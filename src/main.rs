use std::path::PathBuf;

use dact::convert::{self, ActionExtra};
use dact::settings::Settings;
use dact::{APP_NAME, VERSION};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }
    if args.iter().any(|a| a == "--version") {
        println!("{APP_NAME} {VERSION}");
        return;
    }
    if args.iter().any(|a| a == "--selftest") {
        match selftest() {
            Ok(()) => {
                println!("SELFTEST OK");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("SELFTEST FAILED: {e}");
                std::process::exit(1);
            }
        }
    }

    if args.first().map(|s| s.as_str()) == Some("convert") {
        if let Err(e) = cli_convert(&args[1..]) {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }

    dact::setup_logging();
    dact::notify::set_app_id();
    hide_console_if_tray(&args);

    let quiet = args.iter().any(|a| a == "--quiet");
    let files: Vec<PathBuf> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect();

    let payload = if files.is_empty() {
        dact::ipc::IpcMessage {
            files: vec![],
            show: !quiet,
        }
    } else {
        dact::ipc::IpcMessage {
            files: files.clone(),
            show: false,
        }
    };
    if dact::ipc::handoff(&payload) {
        return;
    }

    if let Err(e) = dact::ui::app::run_ui(files, quiet) {
        eprintln!("DAct failed to start: {e}");
        std::process::exit(1);
    }
}

fn cli_convert(args: &[String]) -> anyhow::Result<()> {
    let mut target = None;
    let mut files = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--to" {
            target = args.get(i + 1).cloned();
            i += 2;
            continue;
        }
        if !args[i].starts_with('-') {
            files.push(PathBuf::from(&args[i]));
        }
        i += 1;
    }
    let target = target.ok_or_else(|| anyhow::anyhow!("usage: dact convert <file> --to JPG"))?;
    if files.is_empty() {
        anyhow::bail!("no input files");
    }
    let settings = Settings::load();
    let cat = dact::category::category(&files[0]).ok_or_else(|| anyhow::anyhow!("unsupported file"))?;
    let key = match cat {
        dact::category::Category::Audio | dact::category::Category::Video => format!("media:{target}"),
        _ => format!("convert:{target}"),
    };
    let outs = convert::run_action(&key, &files, &settings, &ActionExtra::default())?;
    for o in outs {
        println!("{}", o.display());
    }
    Ok(())
}

fn selftest() -> anyhow::Result<()> {
    let dir = std::env::temp_dir().join(format!("dact-selftest-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let src = dir.join("t.png");
    let img = image::RgbImage::from_pixel(48, 32, image::Rgb([210, 90, 50]));
    image::DynamicImage::ImageRgb8(img).save(&src)?;
    let settings = Settings::default();
    for t in ["JPG", "WEBP", "BMP", "PDF"] {
        convert::run_action(
            &format!("convert:{t}"),
            &[src.clone()],
            &settings,
            &ActionExtra::default(),
        )?;
    }
    convert::run_action(
        "tool:Resize",
        &[src.clone()],
        &settings,
        &ActionExtra {
            resize: Some(("w".into(), 24)),
            ..Default::default()
        },
    )?;
    convert::run_action(
        "tool:Compress",
        &[src],
        &settings,
        &ActionExtra {
            quality: Some(60),
            ..Default::default()
        },
    )?;
    let txt = dir.join("n.txt");
    std::fs::write(&txt, "hello dact")?;
    convert::run_action("convert:PDF", &[txt], &settings, &ActionExtra::default())?;
    Ok(())
}

fn print_help() {
    println!(
        "{APP_NAME} {VERSION}
Convert and edit files from a radial dial.

Usage:
  dact                     Run the tray app
  dact <files...>          Open the dial for those files
  dact convert <file> --to JPG
  dact --selftest
  dact --quiet             Start in the tray without opening the window

Select a file, hold Alt, press the left mouse button, release on a slice.
Close the window — DAct stays in the tray. Quit from the tray icon.
The original is never changed.
"
    );
}

fn hide_console_if_tray(args: &[String]) {
    let cli = args.iter().any(|a| {
        a == "convert" || a == "--selftest" || a == "--help" || a == "-h" || a == "--version"
    });
    if cli {
        return;
    }
    #[cfg(windows)]
    {
        use windows::Win32::System::Console::GetConsoleWindow;
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
        unsafe {
            let hwnd = GetConsoleWindow();
            if !hwnd.is_invalid() {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        }
    }
}
