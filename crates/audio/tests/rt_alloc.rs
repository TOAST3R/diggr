//! The audio callback must never allocate. This binary installs the counting allocator and
//! drives the renderer through every code path: playback, gapless boundaries, seeks/flushes,
//! EQ changes, underruns, end of queue, tap overflow, pause, and device channel layouts.

use std::sync::Arc;

use audio::PlayState;
use audio::clock::clock;
use audio::eq::{EqCoefs, EqSettings};
use audio::renderer::{Control, Renderer, RendererParts};
use audio::ring::pcm_ring;
use audio::rt_guard::{self, GuardAlloc};
use audio::tap::tap;
use platform::{CallbackInfo, StreamFormat};

#[global_allocator]
static ALLOC: GuardAlloc = GuardAlloc;

/// One test (not two) because the violation counter is process-wide and tests run in parallel.
#[test]
fn renderer_never_allocates_and_guard_works() {
    renderer_never_allocates();

    let before = rt_guard::violations();
    {
        let _rt = rt_guard::enter();
        let v: Vec<u8> = Vec::with_capacity(64);
        std::hint::black_box(&v);
    }
    assert!(
        rt_guard::violations() > before,
        "the guard must notice an allocation"
    );
}

fn renderer_never_allocates() {
    for channels in [1u16, 2, 6] {
        let rate = 48_000;
        let control = Arc::new(Control::default());
        let (mut tx, pcm) = pcm_ring(rate as usize, 512);
        let (mut eq_tx, eq_rx) = rtrb::RingBuffer::new(8);
        let (tap_w, _tap_r) = tap(4); // tiny: overflows quickly, exercising the drop path
        let (clock_w, _clock_r) = clock();
        let mut r = Renderer::new(RendererParts {
            control: control.clone(),
            pcm,
            eq_rx,
            eq: EqCoefs::from_settings(&EqSettings::default(), rate),
            tap: tap_w,
            clock: clock_w,
            format: StreamFormat {
                sample_rate: rate,
                channels,
                buffer_frames: Some(512),
            },
        });
        let mut out = vec![0.0f32; 4096 * channels as usize]; // > BLOCK_FRAMES: multi-block path
        let chunk = vec![0.25f32; 700 * 2];
        let eq_on = EqCoefs::from_settings(
            &EqSettings {
                enabled: true,
                preamp_db: 3.0,
                bands_db: [6.0; 10],
            },
            rate,
        );

        // Everything that could allocate outside the callback happens before the baseline.
        let baseline = rt_guard::violations();
        let mut host = 0u64;
        let mut call = |r: &mut Renderer, out: &mut [f32]| {
            host += 10_000_000;
            r.render(
                out,
                CallbackInfo {
                    host_ns: host,
                    output_latency_ns: 5_000_000,
                },
            );
        };

        call(&mut r, &mut out); // stopped
        let g = control.bump_generation();
        control.set_state(PlayState::Playing);
        call(&mut r, &mut out); // starved while loading
        for i in 0..6 {
            tx.push(g, 1, i * 700, &chunk);
        }
        tx.push(g, 2, 0, &chunk); // gapless boundary
        eq_tx.push(eq_on).unwrap(); // EQ switch + ramp
        control.set_filter(0.4); // the low-pass comes on and sweeps
        call(&mut r, &mut out);
        control.set_filter(1.0); // and fades out again
        control.set_volume(0.3);
        control.set_balance(-0.5);
        call(&mut r, &mut out); // underrun
        let g2 = control.bump_generation(); // seek: stale data flushed
        tx.push(g, 2, 700, &chunk);
        tx.push(g2, 2, 90_000, &chunk);
        call(&mut r, &mut out);
        control.set_state(PlayState::Paused);
        call(&mut r, &mut out);
        control.set_state(PlayState::Playing);
        tx.push(g2, 2, 90_700, &chunk);
        tx.push_end_of_queue(g2, 2, 91_400);
        call(&mut r, &mut out); // end of queue
        control.set_state(PlayState::Stopped);
        call(&mut r, &mut out);

        assert_eq!(
            rt_guard::violations() - baseline,
            0,
            "the render callback allocated ({channels} channels)"
        );
    }
}
