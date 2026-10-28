# Chapter 29: The Factory — figure map

Run with:

```
sim-common/run_sim.sh ch29_factory playground
```

No DUT: the subject is *which type gets built*, not what it drives.

rustdv has a factory, and it works as the UVM factory works (D74). Two
constructors — `Foo::new_comp()` (the normal way, fixed) and
`Foo::create_comp()` (overridable) — plus `Factory::…` to install and inspect
overrides. Every child field a block may want to override is an `RustdvComp`.

| Figure | Title | Where |
|---|---|---|
| 1 | A tiny example component | `TinyComponent` |
| 2 | Building the component the normal way | `TinyTest` (`new_comp`) |
| 4 | Building it through the factory | `TinyFactoryTest` (`create_comp`) |
| 6 | Building from a string name | `CreateByNameTest` |
| 7 | The component we substitute in | `MediumComponent` |
| 8 | Overriding a type, by type | `MediumFactoryTest` |
| 10 | The same override, by name | `MediumNameTest` |
| 12–13 | Overriding one instance | `TwoCompEnv` / `TwoCompTest` |
| 15 | Printing the overrides | `PrintOverridesTest` |

Port of the Python book's chapter 33.

## Transcript (seed 1)

```
      0.00ns INFO     running TinyTest (1/7)  [ch29-factory/src/ch29_factory.rs:71]
      0.00ns INFO     [TinyTest.tc]: I'm so tiny!
      0.00ns INFO     running TinyFactoryTest (2/7)  [ch29-factory/src/ch29_factory.rs:90]
      0.00ns INFO     [TinyFactoryTest.tc]: I'm so tiny!
      0.00ns INFO     running CreateByNameTest (3/7)  [ch29-factory/src/ch29_factory.rs:109]
      0.00ns INFO     [CreateByNameTest.tc]: I'm so tiny!
      0.00ns INFO     running MediumFactoryTest (4/7)  [ch29-factory/src/ch29_factory.rs:151]
      0.00ns INFO     [MediumFactoryTest.tc]: I'm medium size.
      0.00ns INFO     running MediumNameTest (5/7)  [ch29-factory/src/ch29_factory.rs:171]
      0.00ns INFO     [MediumNameTest.tc]: I'm medium size.
      0.00ns INFO     running TwoCompTest (6/7)  [ch29-factory/src/ch29_factory.rs:218]
      0.00ns INFO     [TwoCompTest.env.tc1]: I'm medium size.
      0.00ns INFO     [TwoCompTest.env.tc2]: I'm so tiny!
      0.00ns INFO     running PrintOverridesTest (7/7)  [ch29-factory/src/ch29_factory.rs:241]
      0.00ns INFO     Factory overrides:
      0.00ns INFO       *                           : TinyComponent -> MediumComponent
      0.00ns INFO     [PrintOverridesTest.tc]: I'm medium size.
******************************************************************************
REGRESSION: PASS
```

## What to read the code for

**`new_comp()` vs `create_comp()`.** Figures 2 and 4 have identical structs —
one `RustdvComp` field each — and differ in one line. `new_comp()` builds a
fixed component; `create_comp()` flags the slot, and the build walk swaps in
an override if one is installed. Overridability is the build line, not the
type (D75).

**Instance override, name-free.** `TwoCompEnv` builds `tc1` and `tc2`
identically with `create_comp()` — no names typed. `TwoCompTest` overrides
only `env.tc1`, and the framework tells the two apart because the *fields*
are named `tc1` and `tc2`. The path in `set_inst_override(ctx, "env.tc1")` is
a string only because it addresses a component elsewhere in the tree, exactly
as `ConfigDb::set` does.

**Universal registration.** `#[derive(Component)]` enrols every component by
name (D73), so `create_by_name("TinyComponent")` and
`set_type_override_by_name` work with no separate registration step. 

## Verification

Not vacuous: delete the `set_inst_override` line and both `tc1` and `tc2`
build as `TinyComponent` ("I'm so tiny!"); restore it and only `tc1` becomes
`MediumComponent`. The override drives the outcome.

## Preserve these teaching points

Manuscript: [chapter-29-factory](../../../book-pdf/src/chapter-29-factory.md).

- Preserve the difference between fixed construction (new_comp) and factory
  construction (create_comp), with universal by-name registration for applicable
  derived components.
- A maker closure is how the registry constructs an object; it does not replace
  runtime type/name/instance overrides. Keep those mechanisms distinct.
- Follow the source for generic-component registration exceptions rather than
  inventing a no_factory attribute.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
