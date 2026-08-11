//! Long-running callback ownership regression for simulator-backed tests.

use std::process::Command;

use rustdv::prelude::*;

const DEFAULT_CYCLES: u64 = 10_000;
const DEFAULT_MAX_RSS_GROWTH_KIB: u64 = 16 * 1024;

fn env_u64(name: &str, default: u64) -> Result<u64, TestError> {
    match std::env::var(name) {
        Ok(value) => value.parse::<u64>().map_err(|_| {
            TestError::new(format!("{name} must be an unsigned integer, got '{value}'"))
        }),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(TestError::new(format!("could not read {name}: {error}"))),
    }
}

fn rss_kib() -> Result<u64, TestError> {
    let pid = std::process::id().to_string();
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .output()
        .map_err(|error| TestError::new(format!("could not sample simulator RSS: {error}")))?;
    if !output.status.success() {
        return Err(TestError::new(format!(
            "ps failed while sampling simulator RSS: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .map_err(|error| TestError::new(format!("could not parse simulator RSS: {error}")))
}

#[rustdv::test]
async fn callback_stress_one_shot_handles_plateau(ctx: RustdvCtx) -> Result<(), TestError> {
    let cycles = env_u64("RUSTDV_CALLBACK_STRESS_CYCLES", DEFAULT_CYCLES)?;
    let max_growth = env_u64(
        "RUSTDV_CALLBACK_STRESS_MAX_RSS_GROWTH_KIB",
        DEFAULT_MAX_RSS_GROWTH_KIB,
    )?;
    check!(
        cycles >= 10,
        "callback stress needs at least 10 cycles, got {cycles}"
    );

    let clk = ctx.dut().signal("clk")?;
    Clock::new(&clk, SimDuration::ns(2)).start();
    let warmup_cycle = cycles / 10;
    let mut warmup_rss = None;

    for cycle in 1..=cycles {
        read_only().await;
        next_time_step().await;
        if cycle == warmup_cycle {
            warmup_rss = Some(rss_kib()?);
        }
    }

    let warmup_rss = warmup_rss.expect("warm-up sample was not taken");
    let final_rss = rss_kib()?;
    let growth = final_rss.saturating_sub(warmup_rss);
    rustdv::log::info(&format!(
        "CALLBACK STRESS: cycles={cycles} warmup_rss={warmup_rss}KiB final_rss={final_rss}KiB growth={growth}KiB"
    ));
    check!(
        growth <= max_growth,
        "one-shot callback RSS grew by {growth}KiB after warm-up; limit is {max_growth}KiB"
    );
    Ok(())
}
