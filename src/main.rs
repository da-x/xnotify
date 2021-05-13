#[macro_use]
extern crate error_chain;

use structopt::StructOpt;
use thiserror::Error;
use std::path::PathBuf;
use xcb::{Connection, xproto::Screen, randr};
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

mod leechbar;

#[derive(StructOpt, Debug)]
struct Opt {
    /// Time to wait until message automatically gets off the screen
    #[structopt(long = "timeout", short="t")]
    timeout: Option<f32>,

    /// Take text from file instead of standard input. If file is '-', takes from standard input.
    #[structopt(long = "from-file", short="x")]
    from_file: Option<PathBuf>,

    /// Font to use (Pango font string, for example "normal 100" for big text)
    #[structopt(long = "font", short="n", default_value="9x15bold")]
    font: String,

    /// Make the window flash its colors
    #[structopt(long = "blink", short="l")]
    blink: bool,

    /// Duration of the blink.
    #[structopt(long = "blink-duration", short="d", default_value="0.25")]
    blink_duration: f32,

    /// Rate of the blink (time between each color flip)
    #[structopt(long = "blink-rate", short="e", default_value="0.05")]
    blink_rate: f32,

    /// Initial screen position
    #[structopt(long = "position", short="p", default_value="%50,%50")]
    position: String,
}

#[derive(Error, Debug)]
enum Error {
    #[error("Io error; {0}")]
    IoError(#[from] std::io::Error),
    #[error("ParseInt error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Xcb error; {0}")]
    XcbError(#[from] xcb::Error<xcb::ffi::xcb_generic_error_t>),
    #[error("No screen found")]
    NoScreenFound,
    #[error("Invalid position specified")]
    InvalidPosition,
}

fn main() {
    match main_wrap() {
        Ok(()) => {},
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(-1);
        }
    }
}

fn get_largest_window(conn: &Connection, screen: &Screen) -> Result<((i16, i16), (u16, u16)), Error> {
    let window_dummy = conn.generate_id();

    xcb::create_window(&conn, 0, window_dummy, screen.root(), 0, 0, 1, 1, 0, 0, 0, &[]);
    conn.flush();

    let screen_res_cookie = randr::get_screen_resources(&conn, window_dummy);
    let screen_res_reply = screen_res_cookie.get_reply().unwrap();
    let crtcs = screen_res_reply.crtcs();

    let mut crtc_cookies = Vec::with_capacity(crtcs.len());
    for crtc in crtcs {
        crtc_cookies.push(randr::get_crtc_info(&conn, *crtc, 0));
    }

    let mut res = Err(Error::NoScreenFound);
    let mut size = 0 as u64;

    for crtc_cookie in crtc_cookies.into_iter() {
        if let Ok(reply) = crtc_cookie.get_reply() {
            let pixels = reply.width() as u64 * reply.height()as u64 ;
            if pixels > size {
                size = pixels;
                res = Ok(((reply.x(), reply.y()), (reply.width(), reply.height())));
            }
        }
    }

    xcb::destroy_window(&conn, window_dummy);

    res
}

fn create_gc_32(conn: &Connection, window: u32) -> Result<u32, Error> {
    // First create a dummy pixmap with 32 bit depth
    let pix32 = conn.generate_id();
    xcb::create_pixmap_checked(&conn, 32, pix32, window, 1, 1)
        .request_check()?;

    // Then create a gc from that pixmap
    let gc = conn.generate_id();
    xcb::create_gc_checked(&conn, gc, pix32, &[])
        .request_check()?;

    // Free pixmap after creating the gc
    xcb::free_pixmap_checked(&conn, pix32)
        .request_check()?;

    Ok(gc)
}

fn draw(blink_state: bool, conn: &Connection, win: u32,
    frame: u32, black: u32, window_pict: u32,
    border_size: u16, border_pad: u16, total_width: u16, total_height: u16,
    text: &leechbar::component::text::Text) -> Result<(), Error>
{
    xcb::poly_fill_rectangle(&conn, win, if blink_state { frame } else { black },
        &[xcb::Rectangle::new(
            0, 0,
            total_width + (border_pad * 2 + border_size),
            total_height + (border_pad * 2 + border_size))
        ]);

    xcb::poly_rectangle(&conn, win, frame,
        &[xcb::Rectangle::new(
            0, 0,
            total_width + (border_pad * 2 + border_size),
            total_height + (border_pad * 2 + border_size))
        ]);

    let op = xcb::render::PICT_OP_OVER as u8;
    let pw = text.arc.geometry.width;
    let ph = text.arc.geometry.height;

    xcb::render::composite_checked(
        &conn, op, text.arc.xid, 0, window_pict,
        0, 0, 0, 0,
        (border_pad + border_size) as i16, (border_pad + border_size) as i16, pw, ph
    ).request_check()?;

    conn.flush();

    Ok(())
}

fn parse_position(v: &str, measure: u16, screen_measure: u16) -> Result<i16, Error>
{
    let pos = if v.starts_with("%") {
        let percent : u64 = v[1..].parse()?;
        ((screen_measure as u64 * percent) / 100) as u16
    } else {
        v[..].parse()?
    };
    let pos = pos as i16 - measure as i16 / 2;
    let min = screen_measure.saturating_sub(measure) as i16;
    Ok(std::cmp::max(0, std::cmp::min(min, pos)))
}

fn main_wrap() -> Result<(), Error> {
    let opt = Opt::from_args();

    let (conn, screen_num) = xcb::Connection::connect(None).unwrap();
    let conn = Arc::new(conn);
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();
    let foreground = conn.generate_id();
    let frame = conn.generate_id();
    let black = conn.generate_id();
    let pango_font = pango::FontDescription::from_string(&opt.font);

    let text = if let Some(from_file) = opt.from_file {
        std::fs::read_to_string(from_file)?
    } else {
        let mut buffer = String::new();
        let stdin = std::io::stdin();
        let mut handle = stdin.lock();
        handle.read_to_string(&mut buffer)?;
        buffer
    };

    let (format24, format32) = leechbar::util::formats::image_formats(&conn);
    let (text_width, text_height) =
        leechbar::component::text::text_size(&text, &pango_font).unwrap();
    let border_size = 1;
    let border_pad = 10;

    xcb::create_gc(&conn, foreground, screen.root(), &[
        (xcb::GC_FOREGROUND, screen.white_pixel()),
        (xcb::GC_GRAPHICS_EXPOSURES, 0),
    ]).request_check()?;

    xcb::create_gc(&conn, frame, screen.root(), &[
        (xcb::GC_FOREGROUND, screen.white_pixel()),
        (xcb::GC_GRAPHICS_EXPOSURES, 0),
    ]).request_check()?;

    xcb::create_gc(&conn, black, screen.root(), &[
        (xcb::GC_FOREGROUND, screen.black_pixel()),
        (xcb::GC_GRAPHICS_EXPOSURES, 0),
    ]).request_check()?;

    let largest_window = get_largest_window(&conn, &screen)?;
    let total_width = text_width + (border_pad + border_size) * 2;
    let total_height = text_height + (border_pad + border_size) * 2;

    // Create the window
    let win = conn.generate_id();
    let (mut pos_x, mut pos_y) = if let Some((pos_x, pos_y)) = opt.position.split_once(",") {
        let x = parse_position(pos_x, total_width, largest_window.1.0)?;
        let y = parse_position(pos_y, total_height, largest_window.1.1)?;
        ((largest_window.0.0 + x) as i16, (largest_window.0.1 + y) as i16)
    } else {
        return Err(Error::InvalidPosition);
    };

    xcb::create_window(&conn,
        xcb::WINDOW_CLASS_COPY_FROM_PARENT as u8,
        win,
        screen.root(),
        pos_x,
        pos_y,
        total_width, total_height,
        0,
        xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
        screen.root_visual(), &[
            (xcb::CW_BACK_PIXEL, screen.black_pixel()),
            (xcb::CW_OVERRIDE_REDIRECT, 1),
            (xcb::CW_EVENT_MASK,
             xcb::EVENT_MASK_EXPOSURE |
             xcb::EVENT_MASK_STRUCTURE_NOTIFY |
             xcb::EVENT_MASK_POINTER_MOTION |
             xcb::EVENT_MASK_BUTTON_PRESS |
             xcb::EVENT_MASK_BUTTON_RELEASE),
        ]
    ).request_check()?;

    conn.flush();

    let gcontext = create_gc_32(&conn, win)?;
    let geometry = leechbar::util::Geometry::new(0, 0, text_width, text_height);
    let color = leechbar::util::Color::new(255, 255, 255, 255);
    let text = leechbar::component::text::Text::new(
        conn.clone(), geometry, gcontext, win, format32, &text, &pango_font, color,
    ).unwrap();

    let window_pict = conn.generate_id();
    xcb::render::create_picture_checked(&conn, window_pict, win, format24, &[])
        .request_check()?;

    // Map window while preserving focus on the currently focused application.
    let data = xcb::get_input_focus(&conn);
    let r = data.get_reply()?;
    xcb::map_window(&conn, win).request_check()?;
    xcb::set_input_focus(&conn, r.revert_to(), r.focus(), xcb::CURRENT_TIME).request_check()?;
    conn.flush();

    // Main loop
    let start_time = std::time::Instant::now();
    let mut blink_state = false;

    let mut dur = match opt.timeout {
        Some(t) => Some(Duration::from_millis((1000.0 * t) as u64)),
        None => None,
    };
    let mut grab_pointer_coords = None;
    let mut pending_configure = false;

    while match dur {
        Some(dur) => start_time.elapsed() < dur,
        None => true
    } {
        std::thread::sleep(Duration::from_millis(1));

        if opt.blink {
            let elapsed = start_time.elapsed();

            let new_blink_state = if elapsed.as_millis() <= (opt.blink_duration * 1000.0) as u128 {
                elapsed.div_f32(opt.blink_rate).as_secs() % 2 == 0
            } else {
                false
            };

            if new_blink_state != blink_state {
                blink_state = new_blink_state;
                draw(blink_state, &conn, win, frame, black, window_pict, border_size, border_pad,
                    text_width, text_height, &text)?;
            }
        }

        let event = if let Some(event) = conn.poll_for_event() {
            event
        } else {
            continue;
        };

        let r = event.response_type() & !0x80;
        match r {
            xcb::CONFIGURE_NOTIFY => {
                pending_configure = false;
                draw(blink_state, &conn, win, frame, black, window_pict, border_size, border_pad,
                    text_width, text_height, &text)?;
            },
            xcb::MOTION_NOTIFY => {
                let event: &xcb::MotionNotifyEvent = unsafe { xcb::cast_event(&event) };

                if !pending_configure {
                    if let Some((px, py)) = grab_pointer_coords {
                        let (x, y) = (event.event_x(), event.event_y());

                        if x != px || y != py {
                            pos_x = event.root_x() - px;
                            pos_y = event.root_y() - py;

                            xcb::configure_window(&conn, win, &[
                                (xcb::CONFIG_WINDOW_X as u16, pos_x as u32),
                                (xcb::CONFIG_WINDOW_Y as u16, pos_y as u32),
                            ]).request_check()?;

                            pending_configure = true;
                        }
                    }
                }
            },
            xcb::BUTTON_PRESS => {
                let event: &xcb::ButtonPressEvent = unsafe { xcb::cast_event(&event) };
                let button = event.detail() as u32;
                if button == xcb::BUTTON_INDEX_3 {
                    break;
                } else if button == xcb::BUTTON_INDEX_1 {
                    let event_mask = xcb::EVENT_MASK_POINTER_MOTION
                        | xcb::EVENT_MASK_BUTTON_RELEASE;
                    grab_pointer_coords = Some((event.event_x(), event.event_y()));
                    xcb::grab_pointer(&conn, true, screen.root(), event_mask as u16,
                        xcb::GRAB_MODE_ASYNC as u8, xcb::GRAB_MODE_ASYNC as u8, 0, 0,
                        xcb::CURRENT_TIME);
                    dur = None;
                }
            },
            xcb::BUTTON_RELEASE => {
                let event: &xcb::ButtonReleaseEvent = unsafe { xcb::cast_event(&event) };
                let button = event.detail() as u32;
                if button == xcb::BUTTON_INDEX_1 {
                    xcb::ungrab_pointer(&conn, xcb::CURRENT_TIME);
                    grab_pointer_coords = None;
                }
            },
            xcb::DESTROY_NOTIFY => {
                break;
            },
            _ => {}
        }
    }

    Ok(())
}
