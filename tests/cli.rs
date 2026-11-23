//! End-to-end checks of the `ascii-shadow` binary. URLs are served by a small
//! HTTP server on 127.0.0.1; nothing leaves the machine.

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread;
use std::time::{Duration, Instant};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ascii-shadow"))
        .args(args)
        .output()
        .expect("binary runs")
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn png(w: u32, h: u32, pixel: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    let img = image::RgbImage::from_fn(w, h, |x, y| image::Rgb(pixel(x, y)));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .unwrap();
    bytes
}

/// Two full-canvas frames: white, then black. Loops forever unless `repeat` says otherwise.
fn gif(w: u16, h: u16) -> Vec<u8> {
    let palette = [0, 0, 0, 255, 255, 255];
    let mut out = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut out, w, h, &palette).unwrap();
        encoder.set_repeat(gif::Repeat::Infinite).unwrap();
        for index in [1u8, 0u8] {
            let pixels = vec![index; w as usize * h as usize];
            let frame = gif::Frame {
                width: w,
                height: h,
                delay: 2,
                buffer: std::borrow::Cow::Owned(pixels),
                ..gif::Frame::default()
            };
            encoder.write_frame(&frame).unwrap();
        }
    }
    out
}

fn file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

fn grid(text: &str) -> Vec<String> {
    text.lines().map(str::to_owned).collect()
}

/// Serve one canned response per connection, in order.
fn serve(responses: Vec<Vec<u8>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        for (response, stream) in responses.into_iter().zip(listener.incoming()) {
            let mut stream = stream.unwrap();
            let mut request = Vec::new();
            let mut byte = [0u8; 1];
            while !request.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
                request.push(byte[0]);
            }
            let _ = stream.write_all(&response);
        }
    });
    format!("http://{address}")
}

fn http(status: &str, headers: &[(&str, String)], body: &[u8]) -> Vec<u8> {
    let mut response = format!("HTTP/1.1 {status}\r\nConnection: close\r\n");
    for (name, value) in headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    let mut bytes = response.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

fn image_response(body: &[u8], kind: &str) -> Vec<u8> {
    http(
        "200 OK",
        &[
            ("Content-Type", kind.into()),
            ("Content-Length", body.len().to_string()),
        ],
        body,
    )
}

#[test]
fn version_and_help_exit_successfully() {
    for flag in ["--version", "-V"] {
        let out = run(&[flag]);
        assert!(out.status.success(), "{flag}: {}", stderr(&out));
        assert!(stdout(&out).starts_with("ascii-shadow "), "{flag}");
    }
    let out = run(&["--help"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("USAGE:"));
}

#[test]
fn short_options_work_and_never_panic() {
    let dir = tempfile::tempdir().unwrap();
    let input = file(
        dir.path(),
        "white.png",
        &png(64, 64, |_, _| [255, 255, 255]),
    );
    let out = run(&[
        "-i",
        input.to_str().unwrap(),
        "-w",
        "20",
        "-h",
        "10",
        "-c",
        "blocks",
        "--color-mode",
        "none",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    let lines = grid(&stdout(&out));
    assert_eq!(lines.len(), 10);
    assert!(lines.iter().all(|l| l == &"█".repeat(20)), "{lines:?}");
}

#[test]
fn invalid_values_are_usage_errors() {
    let dir = tempfile::tempdir().unwrap();
    let input = file(dir.path(), "a.png", &png(4, 4, |_, _| [0, 0, 0]));
    let input = input.to_str().unwrap();
    for bad in [
        &["--fps", "0"][..],
        &["--width", "0"],
        &["--contrast", "500"],
        &["--loops", "x"],
    ] {
        let mut args = vec!["-i", input];
        args.extend_from_slice(bad);
        let out = run(&args);
        assert_eq!(out.status.code(), Some(2), "{bad:?}: {}", stderr(&out));
    }
    for bad in [
        &["--charset", "emoji"][..],
        &["--resize", "zoom"],
        &["--background", "xyz"],
    ] {
        let mut args = vec!["-i", input];
        args.extend_from_slice(bad);
        let out = run(&args);
        assert_eq!(out.status.code(), Some(1), "{bad:?}");
        assert!(!stderr(&out).is_empty());
    }
    let out = run(&["-i", "/nonexistent/picture.png"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("Input file not found"));
}

#[test]
fn broken_and_truncated_images_fail_without_panicking() {
    let dir = tempfile::tempdir().unwrap();
    let good = png(32, 32, |x, _| [x as u8 * 8, 0, 0]);
    for (name, bytes) in [
        ("garbage.png", b"definitely not an image".to_vec()),
        ("empty.png", Vec::new()),
        ("half.png", good[..good.len() / 2].to_vec()),
        ("half.gif", gif(8, 8)[..20].to_vec()),
    ] {
        let path = file(dir.path(), name, &bytes);
        let out = run(&["-i", path.to_str().unwrap()]);
        assert_eq!(out.status.code(), Some(1), "{name}: {}", stderr(&out));
        assert!(stderr(&out).contains("Failed"), "{name}: {}", stderr(&out));
    }
}

#[test]
fn proportions_are_kept_in_character_cells() {
    let dir = tempfile::tempdir().unwrap();
    // A square needs twice as many columns as rows.
    let square = file(
        dir.path(),
        "square.png",
        &png(100, 100, |_, _| [255, 255, 255]),
    );
    let out = run(&[
        "-i",
        square.to_str().unwrap(),
        "-w",
        "40",
        "--color-mode",
        "none",
    ]);
    let lines = grid(&stdout(&out));
    assert_eq!((lines[0].chars().count(), lines.len()), (40, 20));
    // A 4:1 picture inside an 80x80 box: 80 columns, 10 rows.
    let wide = file(
        dir.path(),
        "wide.png",
        &png(400, 100, |_, _| [255, 255, 255]),
    );
    let out = run(&[
        "-i",
        wide.to_str().unwrap(),
        "-w",
        "80",
        "-h",
        "80",
        "--color-mode",
        "none",
    ]);
    let lines = grid(&stdout(&out));
    assert_eq!((lines[0].chars().count(), lines.len()), (80, 10));
    // fill and stretch use the whole box.
    for mode in ["fill", "stretch"] {
        let out = run(&[
            "-i",
            wide.to_str().unwrap(),
            "-w",
            "30",
            "-h",
            "12",
            "--resize",
            mode,
            "--color-mode",
            "none",
        ]);
        let lines = grid(&stdout(&out));
        assert_eq!((lines[0].chars().count(), lines.len()), (30, 12), "{mode}");
    }
}

#[test]
fn braille_draws_two_by_four_dots_per_character() {
    let dir = tempfile::tempdir().unwrap();
    // Left half white, right half black.
    let input = file(
        dir.path(),
        "half.png",
        &png(40, 40, |x, _| if x < 20 { [255; 3] } else { [0; 3] }),
    );
    let out = run(&[
        "-i",
        input.to_str().unwrap(),
        "-c",
        "braille",
        "-w",
        "10",
        "--color-mode",
        "none",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    let lines = grid(&stdout(&out));
    assert_eq!(lines.len(), 5);
    for line in lines {
        assert_eq!(line, "⣿⣿⣿⣿⣿⠀⠀⠀⠀⠀");
    }
}

#[test]
fn charsets_use_only_their_documented_characters() {
    let dir = tempfile::tempdir().unwrap();
    let ramp = file(dir.path(), "ramp.png", &png(256, 16, |x, _| [x as u8; 3]));
    let out = run(&[
        "-i",
        ramp.to_str().unwrap(),
        "-c",
        "numbers",
        "-w",
        "64",
        "--color-mode",
        "none",
    ]);
    let text = stdout(&out);
    assert!(
        text.chars()
            .all(|c| c.is_ascii_digit() || " .:\n".contains(c)),
        "{text}"
    );
    assert!(text.chars().filter(|c| c.is_ascii_digit()).count() > 0);
    let out = run(&[
        "-i",
        ramp.to_str().unwrap(),
        "-c",
        "alphanumeric",
        "-w",
        "64",
        "--color-mode",
        "none",
    ]);
    assert!(
        stdout(&out)
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '\n')
    );
}

#[test]
fn plain_output_has_no_escape_codes_and_color_output_resets() {
    let dir = tempfile::tempdir().unwrap();
    let input = file(dir.path(), "red.png", &png(8, 8, |_, _| [255, 0, 0]));
    let plain = stdout(&run(&[
        "-i",
        input.to_str().unwrap(),
        "-w",
        "4",
        "--color",
        "false",
    ]));
    assert!(!plain.contains('\x1b'), "{plain:?}");
    let colored = stdout(&run(&["-i", input.to_str().unwrap(), "-w", "4"]));
    assert!(colored.contains("\x1b[38;2;255;0;0m") && colored.ends_with("\x1b[0m"));
}

#[test]
fn output_files_are_replaced_but_the_input_is_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = png(16, 16, |_, _| [255, 255, 255]);
    let input = file(dir.path(), "in.png", &bytes);
    let target = file(dir.path(), "art.txt", b"old contents");
    let out = run(&[
        "-i",
        input.to_str().unwrap(),
        "-w",
        "8",
        "-o",
        target.to_str().unwrap(),
        "--color-mode",
        "none",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "@@@@@@@@\n@@@@@@@@\n@@@@@@@@\n@@@@@@@@\n"
    );
    let out = run(&["-i", input.to_str().unwrap(), "-o", input.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("Refusing to overwrite the input"));
    assert_eq!(
        std::fs::read(&input).unwrap(),
        bytes,
        "the input image is untouched"
    );
}

#[test]
fn a_gif_with_output_saves_its_first_frame() {
    let dir = tempfile::tempdir().unwrap();
    let input = file(dir.path(), "anim.gif", &gif(8, 8));
    let target = dir.path().join("frame.txt");
    let out = run(&[
        "-i",
        input.to_str().unwrap(),
        "-w",
        "4",
        "-o",
        target.to_str().unwrap(),
        "--color-mode",
        "none",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "@@@@\n@@@@\n");
}

#[test]
fn a_gif_plays_every_frame_for_the_requested_loops() {
    let dir = tempfile::tempdir().unwrap();
    // The file asks to loop forever; --loops 2 must stop after two plays.
    let input = file(dir.path(), "anim.gif", &gif(8, 8));
    let started = Instant::now();
    let out = run(&[
        "-i",
        input.to_str().unwrap(),
        "-w",
        "4",
        "--loops",
        "2",
        "--fps",
        "100",
        "--color-mode",
        "none",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(started.elapsed() < Duration::from_secs(20));
    let text = stdout(&out);
    assert_eq!(text.matches("@@@@\n@@@@\n").count(), 2, "white frame twice");
    assert_eq!(text.matches("    \n    \n").count(), 2, "black frame twice");
}

#[test]
fn urls_follow_redirects_and_are_recognised_as_gifs_by_content() {
    let image = png(16, 16, |_, _| [255, 255, 255]);
    let base = serve(vec![
        http(
            "302 Found",
            &[
                ("Location", "/real.png".into()),
                ("Content-Length", "0".into()),
            ],
            b"",
        ),
        image_response(&image, "image/png"),
    ]);
    let out = run(&[
        "-i",
        &format!("{base}/start"),
        "-w",
        "4",
        "--color-mode",
        "none",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "@@@@\n@@@@\n");

    // A GIF URL with a query string, served as octet-stream, still animates.
    let base = serve(vec![image_response(&gif(8, 8), "application/octet-stream")]);
    let out = run(&[
        "-i",
        &format!("{base}/a.gif?v=2"),
        "-w",
        "4",
        "--loops",
        "1",
        "--fps",
        "100",
        "--color-mode",
        "none",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out).matches("@@@@\n@@@@\n").count(), 1);
}

#[test]
fn url_errors_are_reported_without_panicking() {
    let cases = [
        (
            http("404 Not Found", &[("Content-Length", "0".into())], b""),
            "HTTP error: 404",
        ),
        (
            image_response(b"<html>nope</html>", "text/html"),
            "did not return an image",
        ),
        (
            http(
                "200 OK",
                &[
                    ("Content-Type", "image/png".into()),
                    ("Content-Length", "1000000000".into()),
                ],
                b"",
            ),
            "larger than 64 MiB",
        ),
        (image_response(b"not really a png", "image/png"), "decode"),
    ];
    for (response, expected) in cases {
        let base = serve(vec![response]);
        let out = run(&["-i", &format!("{base}/x.png")]);
        assert_eq!(out.status.code(), Some(1), "{expected}: {}", stderr(&out));
        assert!(
            stderr(&out).contains(expected),
            "expected {expected:?} in {}",
            stderr(&out)
        );
    }
}

#[test]
fn downloads_stop_at_the_size_limit_and_the_timeout() {
    // No Content-Length: the limit applies while reading.
    let base = serve(vec![http(
        "200 OK",
        &[("Content-Type", "image/png".into())],
        &vec![0u8; 4096],
    )]);
    let err = ascii_shadow::image::fetch_url(&format!("{base}/big"), 1024, Duration::from_secs(5))
        .unwrap_err();
    assert!(err.to_string().contains("larger than"), "{err}");

    // A server that accepts and never answers.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        let _held: Vec<_> = listener.incoming().take(1).collect();
        thread::sleep(Duration::from_secs(10));
    });
    let started = Instant::now();
    let result = ascii_shadow::image::fetch_url(
        &format!("http://{address}/slow"),
        1024,
        Duration::from_millis(500),
    );
    assert!(result.is_err());
    assert!(started.elapsed() < Duration::from_secs(5));
}
