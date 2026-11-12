use std::f32::consts::TAU;
use std::ffi::{c_char, c_int, c_void};
use std::io::Write;
use std::process::{Command, Stdio};
use std::ptr;

const RATE: u32 = 48_000;
const PULSE_PLAYBACK: c_int = 1;
const PULSE_S16LE: c_int = 3;

pub enum Feedback {
    Capture,
    Copy,
}

impl Feedback {
    pub fn play(self) {
        let _ = std::thread::Builder::new()
            .name("color-picker-audio".into())
            .spawn(move || {
                let samples = self.samples();
                if !play_pulse(&samples).unwrap_or(false)
                    && !play_command(
                        "pw-play",
                        &["--raw", "--rate=48000", "--channels=1", "--format=s16", "-"],
                        &samples,
                    )
                    && !play_command(
                        "aplay",
                        &["-q", "-t", "raw", "-r", "48000", "-c", "1", "-f", "S16_LE"],
                        &samples,
                    )
                {
                    eprintln!(
                        "Could not play color picker feedback through the system audio output."
                    );
                }
            });
    }

    fn samples(&self) -> Vec<u8> {
        let (start, end, duration) = match self {
            Self::Capture => (1_200.0, 480.0, 0.12),
            Self::Copy => (720.0, 1_050.0, 0.10),
        };
        let sample_count = (RATE as f32 * duration) as usize;
        let mut phase = 0.0;
        (0..sample_count)
            .flat_map(|index| {
                let progress = index as f32 / sample_count as f32;
                let frequency = start + (end - start) * progress;
                phase += TAU * frequency / RATE as f32;
                let envelope = (progress * 25.0).min(1.0) * (1.0 - progress).powi(2);
                let sample = (phase.sin() * envelope * 0.28 * f32::from(i16::MAX)) as i16;
                sample.to_le_bytes()
            })
            .collect()
    }
}

#[repr(C)]
struct PulseSampleSpec {
    format: c_int,
    rate: u32,
    channels: u8,
}

type OpenStream = unsafe extern "C" fn(
    *const c_char,
    *const c_char,
    c_int,
    *const c_char,
    *const c_char,
    *const PulseSampleSpec,
    *const c_void,
    *const c_void,
    *mut c_int,
) -> *mut c_void;
type WriteStream = unsafe extern "C" fn(*mut c_void, *const c_void, usize, *mut c_int) -> c_int;
type DrainStream = unsafe extern "C" fn(*mut c_void, *mut c_int) -> c_int;
type FreeStream = unsafe extern "C" fn(*mut c_void);

fn play_pulse(samples: &[u8]) -> Result<bool, libloading::Error> {
    unsafe {
        let pulse = libloading::Library::new("libpulse-simple.so.0")?;
        let open = pulse.get::<OpenStream>(b"pa_simple_new")?;
        let write = pulse.get::<WriteStream>(b"pa_simple_write")?;
        let drain = pulse.get::<DrainStream>(b"pa_simple_drain")?;
        let free = pulse.get::<FreeStream>(b"pa_simple_free")?;
        let spec = PulseSampleSpec {
            format: PULSE_S16LE,
            rate: RATE,
            channels: 1,
        };
        let stream = open(
            ptr::null(),
            c"Luti Color Picker".as_ptr(),
            PULSE_PLAYBACK,
            ptr::null(),
            c"Capture and copy feedback".as_ptr(),
            &spec,
            ptr::null(),
            ptr::null(),
            ptr::null_mut(),
        );
        if stream.is_null() {
            return Ok(false);
        }
        let played = write(
            stream,
            samples.as_ptr().cast(),
            samples.len(),
            ptr::null_mut(),
        ) == 0
            && drain(stream, ptr::null_mut()) == 0;
        free(stream);
        Ok(played)
    }
}

fn play_command(player: &str, args: &[&str], samples: &[u8]) -> bool {
    let Ok(mut child) = Command::new(player)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let written = child
        .stdin
        .take()
        .is_some_and(|mut input| input.write_all(samples).is_ok());
    child.wait().is_ok_and(|status| status.success()) && written
}
