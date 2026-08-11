# `tinyalu_tb` — the shipped TinyALU testbench

The complete testbench, and the source for **the Interlude** and **Chapter 40**.
It is not a book-figure crate: there are no `// Chapter N, Figure M:` captions
here, because the Interlude presents it as excerpts and ch40 walks it as a whole.

Run with:

```
sim/run_rustdv.sh          # or: sim/run_rustdv.sh release
sim/run_rustdv.sh release verilator
```

Its files are `tinyalu_tb.rs` (crate root — there is no `lib.rs`, D29), `env.rs`,
`components.rs`, `sequences.rs`, `alu_item.rs` and `alu_bfm.rs`.

Two tests, ending `REGRESSION: PASS`: **RandomTest** — 20 compared, 0 mismatches,
coverage Add=5 And=5 Mul=5 Xor=5; **MaxTest** — 4 compared, 0 mismatches, every
operation covered.

## Transcript

Verbatim from `sim/run_rustdv.sh release icarus`, `RUSTDV_RANDOM_SEED=1`, Linux/Icarus,
2026-07-30. This is the block the Interlude and ch40 both draw from; the cargo
build lines above the first `0.00ns` are omitted.

```
      0.00ns INFO     rustdv: found 2 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running RandomTest (1/2)  [tinyalu_tb/src/tinyalu_tb.rs:77]
     70.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 296 }
     70.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 193, b: 103, op: Add }
     90.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 10 }
     90.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 94, b: 11, op: And }
    110.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 57 }
    110.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 185, b: 128, op: Xor }
    130.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 165, b: 117, op: Mul }
    160.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 19305 }
    180.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 168, b: 150, op: Add }
    180.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 318 }
    200.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 97, b: 254, op: And }
    200.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 96 }
    220.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 192, b: 138, op: Xor }
    220.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 74 }
    240.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 168, b: 59, op: Mul }
    270.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 9912 }
    290.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 340 }
    290.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 99, b: 241, op: Add }
    310.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 8 }
    310.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 238, b: 8, op: And }
    330.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 218 }
    330.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 70, b: 156, op: Xor }
    350.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 205, b: 172, op: Mul }
    380.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 35260 }
    400.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 159, b: 247, op: Add }
    400.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 406 }
    420.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 53, b: 171, op: And }
    420.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 33 }
    440.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 39, b: 138, op: Xor }
    440.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 173 }
    460.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 132, b: 186, op: Mul }
    490.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 24552 }
    510.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 137 }
    510.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 109, b: 28, op: Add }
    530.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 4 }
    530.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 23, b: 12, op: And }
    550.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 52 }
    550.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 245, b: 193, op: Xor }
    570.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 24, b: 60, op: Mul }
    600.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 1440 }
    630.00ns INFO     [RandomTest.inner]: sequence complete
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 193, b: 103, op: Add } out=AluResult { result: 296 } expected=AluResult { result: 296 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 94, b: 11, op: And } out=AluResult { result: 10 } expected=AluResult { result: 10 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 185, b: 128, op: Xor } out=AluResult { result: 57 } expected=AluResult { result: 57 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 165, b: 117, op: Mul } out=AluResult { result: 19305 } expected=AluResult { result: 19305 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 168, b: 150, op: Add } out=AluResult { result: 318 } expected=AluResult { result: 318 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 97, b: 254, op: And } out=AluResult { result: 96 } expected=AluResult { result: 96 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 192, b: 138, op: Xor } out=AluResult { result: 74 } expected=AluResult { result: 74 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 168, b: 59, op: Mul } out=AluResult { result: 9912 } expected=AluResult { result: 9912 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 99, b: 241, op: Add } out=AluResult { result: 340 } expected=AluResult { result: 340 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 238, b: 8, op: And } out=AluResult { result: 8 } expected=AluResult { result: 8 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 70, b: 156, op: Xor } out=AluResult { result: 218 } expected=AluResult { result: 218 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 205, b: 172, op: Mul } out=AluResult { result: 35260 } expected=AluResult { result: 35260 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 159, b: 247, op: Add } out=AluResult { result: 406 } expected=AluResult { result: 406 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 53, b: 171, op: And } out=AluResult { result: 33 } expected=AluResult { result: 33 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 39, b: 138, op: Xor } out=AluResult { result: 173 } expected=AluResult { result: 173 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 132, b: 186, op: Mul } out=AluResult { result: 24552 } expected=AluResult { result: 24552 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 109, b: 28, op: Add } out=AluResult { result: 137 } expected=AluResult { result: 137 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 23, b: 12, op: And } out=AluResult { result: 4 } expected=AluResult { result: 4 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 245, b: 193, op: Xor } out=AluResult { result: 52 } expected=AluResult { result: 52 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 24, b: 60, op: Mul } out=AluResult { result: 1440 } expected=AluResult { result: 1440 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: 20 compared, 0 mismatches
    630.00ns INFO     [RandomTest.inner.env.coverage]: coverage: Add=5 And=5 Mul=5 Xor=5
    630.00ns INFO     RandomTest PASSED
    630.00ns INFO     running MaxTest (2/2)  [tinyalu_tb/src/tinyalu_tb.rs:92]
    700.00ns INFO     [MaxTest.inner.env.result_mon]: result_monitor: AluResult { result: 510 }
    700.00ns INFO     [MaxTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 255, b: 255, op: Add }
    720.00ns INFO     [MaxTest.inner.env.result_mon]: result_monitor: AluResult { result: 255 }
    720.00ns INFO     [MaxTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 255, b: 255, op: And }
    740.00ns INFO     [MaxTest.inner.env.result_mon]: result_monitor: AluResult { result: 0 }
    740.00ns INFO     [MaxTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 255, b: 255, op: Xor }
    760.00ns INFO     [MaxTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 255, b: 255, op: Mul }
    790.00ns INFO     [MaxTest.inner.env.result_mon]: result_monitor: AluResult { result: 65025 }
    820.00ns INFO     [MaxTest.inner]: sequence complete
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 255, b: 255, op: Add } out=AluResult { result: 510 } expected=AluResult { result: 510 } check=PASS
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 255, b: 255, op: And } out=AluResult { result: 255 } expected=AluResult { result: 255 } check=PASS
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 255, b: 255, op: Xor } out=AluResult { result: 0 } expected=AluResult { result: 0 } check=PASS
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 255, b: 255, op: Mul } out=AluResult { result: 65025 } expected=AluResult { result: 65025 } check=PASS
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: 4 compared, 0 mismatches
    820.00ns INFO     [MaxTest.inner.env.coverage]: coverage: Add=1 And=1 Mul=1 Xor=1
    820.00ns INFO     MaxTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** RandomTest                                   PASS         630.00      **
** MaxTest                                      PASS         190.00      **
******************************************************************************
REGRESSION: PASS
```
