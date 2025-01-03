use cache_manager::CacheManager;
use chientrm_youtube_dl::{download_yt_dlp, YoutubeDl};
use clap::{Parser, Subcommand};
use ffmpeg_next::{
    self as ffmpeg,
    format::Pixel,
    frame::Video,
    software::scaling::{Context, Flags},
};
use icy_sixel::{
    sixel_string, DiffusionMethod, MethodForLargest, MethodForRep, PixelFormat, Quality,
};
use image::{ImageBuffer, Rgb};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{sync::mpsc, thread, time::Instant};
use url::Url;
use workspace::get_out_dir;

pub mod cache_manager;
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
    Play {
        query: Url,
    },
    Test {
        #[arg(long)]
        no_cache: bool,
    },
}

async fn download_video(url: Url) -> String {
    let bin_path = workspace::get_bin_dir();
    download_yt_dlp(&bin_path)
        .await
        .expect("Failed to download video");

    let yt_dlp_path = bin_path.join("yt-dlp.exe");
    let mut ytdl = YoutubeDl::new(url);
    ytdl.socket_timeout("8")
        .youtube_dl_path(yt_dlp_path)
        .download_to_async(get_out_dir())
        .await
        .expect("Failed to download video");

    let video = ytdl.run().unwrap().into_single_video().unwrap();

    workspace::get_out_dir()
        .read_dir()
        .unwrap()
        .find(|entry| {
            if let Ok(entry) = entry {
                let entry_file_name = entry.file_name().to_string_lossy().to_string();

                entry_file_name.contains(video.title.as_ref().unwrap().as_str())
                    && entry_file_name.contains("mp4")
            } else {
                false
            }
        })
        .unwrap()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .unwrap()
}

async fn retrieve_video(url: Url, use_cache: bool) -> std::path::PathBuf {
    if !use_cache || !CacheManager::contains(&url) {
        let video_file_name = download_video(url.clone()).await;
        CacheManager::add_record(url.clone(), video_file_name);
    }

    CacheManager::get_video_path(&url)
}

async fn play_video(url: Url, use_cache: bool) {
    let video_path = retrieve_video(url, use_cache).await;

    let mut context = ffmpeg::format::input(&video_path).expect("unable to get context");

    let input = context
        .streams()
        .best(ffmpeg::media::Type::Video)
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
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    )
    .unwrap();

    fn process_frame(mut rgb_frame: Video) {
        let start = Instant::now();

        let sixel_frame = rgb_frame_to_sixel(rgb_frame);
        print!("\x1b[{};{}H", 1, 1);
        println!("{sixel_frame}");

        let elapsed = start.elapsed();
        eprintln!("process_frame took {:.2?}", elapsed);
    }

    let (frame_sender, frame_receiver) = mpsc::sync_channel(64);

    let processor_handle = thread::spawn(move || {
        frame_receiver.into_iter().par_bridge().for_each(|frame| {
            process_frame(frame);
        });
    });

    let mut receive_and_send_frames =
        |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
            let mut decoded_frames = Vec::new();
            let mut decoded = Video::empty();

            while decoder.receive_frame(&mut decoded).is_ok() {
                let mut rgb_frame = Video::empty();
                scaler.run(&decoded, &mut rgb_frame)?;
                decoded_frames.push(rgb_frame);

                if decoded_frames.len() >= 10 {
                    break;
                }
            }

            for frame in decoded_frames.drain(..) {
                frame_sender.send(frame).unwrap();
            }

            Ok(())
        };

    for (stream, packet) in context.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet).unwrap();
            receive_and_send_frames(&mut decoder).unwrap();
        }
    }

    decoder.send_eof().unwrap();
    receive_and_send_frames(&mut decoder).unwrap();

    drop(frame_sender);

    processor_handle.join().unwrap();
}

fn rgb_frame_to_sixel(frame: Video) -> String {
    let width = frame.width();
    let height = frame.height();
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, frame.data(0).to_vec()).unwrap();

    sixel_string(
        &img,
        width as i32,
        height as i32,
        PixelFormat::RGB888,
        DiffusionMethod::Auto,
        MethodForLargest::Auto,
        MethodForRep::Pixels,
        Quality::HIGH,
    )
    .expect("Failed to encode image to SIXEL format")
}

async fn handle_command(command: Command) {
    match command {
        Command::Play { query } => play_video(query, true).await,
        Command::Test { no_cache } => {
            let test_url = Url::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
            play_video(test_url, !no_cache).await;
        }
    }
}

#[tokio::main]
async fn main() {
    CacheManager::initialize();
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        handle_command(command).await;
    } else {
        eprintln!("No command provided");
    }
}
