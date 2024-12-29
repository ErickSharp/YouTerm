use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;
use chientrm_youtube_dl::download_yt_dlp;
use chientrm_youtube_dl::YoutubeDl;
use clap::Parser;
use clap::Subcommand;
use icy_sixel::sixel_string;
use icy_sixel::DiffusionMethod;
use icy_sixel::MethodForLargest;
use icy_sixel::MethodForRep;
use icy_sixel::PixelFormat;
use icy_sixel::Quality;
use url::Url;
use workspace::get_out_dir;
pub mod panic;
pub mod workspace;
#[derive(Debug, Parser)]
struct Cli {
    #[arg(short, long, global = true)]
    verbose: bool,
    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
enum Command {
    Play { query: Url },
    Test,
}

pub fn load_png_as_rgb888(file_path: &str) -> (Vec<u8>, u32, u32) {
    let file = include_bytes!("../Logo.png");
    let cursor = std::io::Cursor::new(file);
    let reader = BufReader::new(cursor);
    let img = image::load(reader, image::ImageFormat::Png).expect("Failed to load PNG image");
    let img_rgb8 = img.to_rgb8(); // Discard alpha channel if present
    let (width, height) = img_rgb8.dimensions();
    let img_rgb888 = img_rgb8.into_raw();
    (img_rgb888, width, height)
}
async fn play_video(url: Url) {
    let (img_rgb888, width, height) = load_png_as_rgb888("./Logo.png");
    // We need to be able to play the video and audio in sync

    // For playing the video, we'll probably store a temporary buffer of a couple frames that we can work with and play sequentially in a well-timed and well-orchestrated fashion
    // For each frame, we need to take it and break it up into the 6 pixel high groups and convert them to sixel blocks. We also need to figure out their color
    
    let url = url.to_string();
    println!("playing: {url}");
    let sixel_data = sixel_string(
        &img_rgb888,
        width as i32,
        height as i32,
        PixelFormat::RGB888,
        DiffusionMethod::Auto, // Auto, None, Atkinson, FS, JaJuNi, Stucki, Burkes, ADither, XDither
        MethodForLargest::Auto, // Auto, Norm, Lum
        MethodForRep::Pixels,    // Auto, CenterBox, AverageColors, Pixels
        Quality::HIGH,         // AUTO, HIGH, LOW, FULL, HIGHCOLOR
    )
    .expect("Failed to encode image to SIXEL format");
    println!("sixel if your terminal knows how to print it:\n{}", sixel_data);

    let  bin_path = workspace::get_bin_dir();
    download_yt_dlp(&bin_path).await.expect("Failed to download video");
    
    let yt_dlp_path = bin_path.join("yt-dlp.exe");
    println!("{}", yt_dlp_path.display());
    let output = YoutubeDl::new(url)
    .socket_timeout("15")
    .youtube_dl_path(yt_dlp_path)
    .download_to_async(get_out_dir()).await.expect("Failed to download video asynchronously");
}

async fn handle_command(command: Command) {
    match command {
        Command::Play { query } => play_video(query),
        Command::Test => play_video(Url::from_str("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap()),
    }.await
}

#[tokio::main]
async fn main() {
    panic::set_hook();
    let cli = Cli::try_parse();

    match cli {
        Ok(cli) => {
            if let Some(command) = cli.command {
                println!("Command: {:#?}", command);
                handle_command(command).await;
            } else {
                eprintln!("No command provided");
            }
        }
        Err(e) => {
            println!("Error type: {}", e.kind());
            e.print().expect("Failed to print error message");
        }
    }
}