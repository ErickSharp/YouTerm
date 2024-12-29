use chientrm_youtube_dl::download_yt_dlp;
use chientrm_youtube_dl::YoutubeDl;
use clap::Parser;
use clap::Subcommand;
use ffmpeg_next::format::Pixel;
use ffmpeg_next::frame;
use ffmpeg_next::frame::Video;
use ffmpeg_next::software::scaling::Context;
use ffmpeg_next::software::scaling::Flags;
use icy_sixel::sixel_string;
use icy_sixel::DiffusionMethod;
use icy_sixel::MethodForLargest;
use icy_sixel::MethodForRep;
use icy_sixel::PixelFormat;
use icy_sixel::Quality;
use image::ImageBuffer;
use image::Rgb;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;
use url::Url;
use workspace::get_out_dir;
pub mod panic;
pub mod workspace;
use ffmpeg_next as ffmpeg;

#[derive(Debug, Parser)]
struct Cli {
    #[arg(short, long, global = true)]
    verbose: bool,
    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
enum Command {
    Play {
        query: Url,
    },
    Test {
        #[arg(long)]
        no_cache: bool,
    },
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

fn basic_test_func(url: Url) {
    let (img_rgb888, width, height) = load_png_as_rgb888("./Logo.png");

    let url = url.to_string();
    println!("playing: {url}");
    let sixel_data = sixel_string(
        &img_rgb888,
        width as i32,
        height as i32,
        PixelFormat::RGB888,
        DiffusionMethod::Auto, // Auto, None, Atkinson, FS, JaJuNi, Stucki, Burkes, ADither, XDither
        MethodForLargest::Auto, // Auto, Norm, Lum
        MethodForRep::Pixels,  // Auto, CenterBox, AverageColors, Pixels
        Quality::HIGH,         // AUTO, HIGH, LOW, FULL, HIGHCOLOR
    )
    .expect("Failed to encode image to SIXEL format");
    println!(
        "sixel if your terminal knows how to print it:\n{}",
        sixel_data
    );
}
async fn play_video(url: Url, use_cache: bool) {
    basic_test_func(url.clone());

    let bin_path = workspace::get_bin_dir();
    download_yt_dlp(&bin_path)
        .await
        .expect("Failed to download video");

    let yt_dlp_path = bin_path.join("yt-dlp.exe");

    let mut ytdl = YoutubeDl::new(url);

    if !use_cache {
        ytdl.socket_timeout("8")
            .youtube_dl_path(yt_dlp_path)
            .download_to_async(get_out_dir())
            .await
            .expect("Failed to download video");
    }

    let video = ytdl.run().unwrap().into_single_video().unwrap();

    let video_path = fs::read_dir(workspace::get_out_dir())
        .unwrap()
        .find(|path| {
            if let Ok(entry) = path {
                let entry_file_name = entry.file_name().to_string_lossy().into_owned();

                entry_file_name.contains(video.title.as_ref().unwrap())
                    && entry_file_name.contains("mp4")
            } else {
                false
            }
        })
        .unwrap()
        .unwrap();

    dbg!(&video_path);
    dbg!(fs::read_dir(workspace::get_out_dir()).unwrap());
    let mut context = ffmpeg::format::input(&video_path.path()).expect("unable to get context");

    let input = context
        .streams()
        .best(ffmpeg_next::media::Type::Video)
        .expect("Could not find video stream");

    let video_stream_index = input.index();

    let context_decoder =
        ffmpeg::codec::context::Context::from_parameters(input.parameters()).unwrap();
    let mut decoder = context_decoder.decoder().video().unwrap();

    let mut scaler = Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        Pixel::RGB24,
        decoder.width() / 4,
        decoder.height() / 4,
        Flags::BILINEAR,
    )
    .unwrap();

    let mut receive_and_process_decoded_frames =
        |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
            let mut decoded = Video::empty();
            while decoder.receive_frame(&mut decoded).is_ok() {
                let mut rgb_frame = Video::empty();
                scaler.run(&decoded, &mut rgb_frame)?;
                let sixel_frame = rgb_frame_to_sixel(rgb_frame);

                print!("\x1b[{};{}H", 1, 1);
                println!("{sixel_frame}");
            }
            Ok(())
        };

    for (stream, packet) in context.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet).unwrap();
            receive_and_process_decoded_frames(&mut decoder).unwrap();
        }
    }
    decoder.send_eof().unwrap();
    receive_and_process_decoded_frames(&mut decoder).unwrap();
}

fn rgb_frame_to_sixel(frame: Video) -> String {
    let width = frame.width();
    let height = frame.height();
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, frame.data(0).to_vec()).unwrap();

    let sixel_data = sixel_string(
        &img,
        width as i32,
        height as i32,
        PixelFormat::RGB888,
        DiffusionMethod::Auto, // Auto, None, Atkinson, FS, JaJuNi, Stucki, Burkes, ADither, XDither
        MethodForLargest::Auto, // Auto, Norm, Lum
        MethodForRep::Pixels,  // Auto, CenterBox, AverageColors, Pixels
        Quality::HIGH,         // AUTO, HIGH, LOW, FULL, HIGHCOLOR
    )
    .expect("Failed to encode image to SIXEL format");

    sixel_data
}

async fn handle_command(command: Command) {
    match command {
        Command::Play { query } => play_video(query, true),
        Command::Test { no_cache } => play_video(
            Url::from_str("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
            no_cache,
        ),
    }
    .await
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
