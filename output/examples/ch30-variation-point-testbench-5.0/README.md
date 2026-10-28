# Chapter 30: Variation-Point Testbench: 5.0 — figure map

Run with:

```
sim-common/run_sim.sh ch30_variation_point_testbench_5_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

| Figure | Title | Where |
|---|---|---|
| 1 | The Tester trait — one method varies, the rest is shared | `src/ch30_variation_point_testbench_5_0.rs` (`Tester`) |
| 2 | The abstract base and the two testers that fill its slot | `src/ch30_variation_point_testbench_5_0.rs` (`BaseTester`/`RandomTester`/`MaxTester`) |
| 3 | The environment builds its tester through the factory | `src/ch30_variation_point_testbench_5_0.rs` (`AluEnv`) |
| 4 | random_test overrides BaseTester with RandomTester | `src/ch30_variation_point_testbench_5_0.rs` |
| 5 | max_test differs only in the tester it installs | `src/ch30_variation_point_testbench_5_0.rs` |
| 6 | One env, two behaviors | transcript |

The tester — the type parameter `AluEnv<T>` of testbench 4.0 (D28) — is now
chosen at run time by a factory type override (D69–D75), the port of the
Python book's chapter 34. The testers themselves are testbench 2.0's
`RandomTester` and `MaxTester` on Chapter 20's `Tester` trait, now also
components (D115). Both tests end `REGRESSION: PASS`
(RUSTDV_RANDOM_SEED=1); results match testbench 4.0's bit for bit.

## Transcript

Real Icarus output (`RUSTDV_RANDOM_SEED=1`):

```
      0.00ns INFO     rustdv: found 2 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running RandomTest (1/2)  [ch30-variation-point-testbench-5.0/src/ch30_variation_point_testbench_5_0.rs:245]
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: c1 Add 67 = 0128
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: 5e And 0b = 000a
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: b9 Xor 80 = 0039
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: a5 Mul 75 = 4b69
    150.00ns INFO     [RandomTest.env.scoreboard]: Covered all operations
    150.00ns INFO     RandomTest PASSED
    150.00ns INFO     running MaxTest (2/2)  [ch30-variation-point-testbench-5.0/src/ch30_variation_point_testbench_5_0.rs:262]
    300.00ns INFO     [MaxTest.env.scoreboard]: PASSED: ff Add ff = 01fe
    300.00ns INFO     [MaxTest.env.scoreboard]: PASSED: ff And ff = 00ff
    300.00ns INFO     [MaxTest.env.scoreboard]: PASSED: ff Xor ff = 0000
    300.00ns INFO     [MaxTest.env.scoreboard]: PASSED: ff Mul ff = fe01
    300.00ns INFO     [MaxTest.env.scoreboard]: Covered all operations
    300.00ns INFO     MaxTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** RandomTest                                   PASS         150.00      **
** MaxTest                                      PASS         150.00      **
******************************************************************************
REGRESSION: PASS
```

The operands and results are identical to testbench 4.0's — the same
stimulus, selected by a runtime factory override instead of a compile-time
type parameter.

## Preserve these teaching points

Manuscript: [chapter-30-variation-point-testbench-5.0](../../../book-pdf/src/chapter-30-variation-point-testbench-5.0.md).

- Runtime variation uses an erased create_comp slot and a factory override.
  Generic parameters express compile-time variation; an override cannot replace a
  type parameter.
- The testers remain the Tester/RandomTester/MaxTester structure the reader met in
  Chapter 20, now also components. Do not introduce an unrelated free-
  function/closure helper just to explain factory variation.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
