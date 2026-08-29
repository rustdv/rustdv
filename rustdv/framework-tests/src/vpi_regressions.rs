//! Focused regressions for simulator-specific VPI object discovery.

use rustdv::prelude::*;

// A compilation-unit SystemVerilog class produces a synthetic Verilator
// top-level scope whose name contains `_Vclpkg`.  The runner must construct
// the context around the requested module, not that generated scope.
#[rustdv::test]
async fn vpi_class_top_scope_selects_dut(ctx: RustdvCtx) -> Result<(), TestError> {
    let top_names: Vec<String> = rustdv::gpi::top_modules()
        .into_iter()
        .map(|top| top.name())
        .collect();
    rustdv::log::info(&format!("VPI TOPS: {top_names:?}"));
    let dut = ctx.dut();
    check!(
        dut.name() == "class_top",
        "selected {:?} as the DUT, not class_top",
        dut.name()
    );

    let input = dut.signal("data_i")?;
    let output = dut.signal("data_o")?;
    input.set_u64(0xa5);
    read_write().await;
    read_only().await;
    check!(
        output.get_u64() == Ok(0xa5),
        "class_top data path did not settle to 0xa5"
    );
    Ok(())
}

// Verilator reports aggregate struct ports as vpiStructVar rather than
// vpiPort, vpiNet, or vpiReg. rustdv should still expose the named port
// objects as LogicHandles. Verilator reports their aggregate width as zero,
// so this deliberately tests discovery rather than packed-value operations.
#[rustdv::test]
async fn vpi_struct_top_ports_are_logic(ctx: RustdvCtx) -> Result<(), TestError> {
    let dut = ctx.dut();
    check!(
        dut.name() == "struct_ports_top",
        "selected {:?} as the DUT, not struct_ports_top",
        dut.name()
    );

    let request = dut.signal("request")?;
    let response = dut.signal("response")?;
    check!(
        request.full_name().ends_with(".request"),
        "request resolved to {:?}",
        request.full_name()
    );
    check!(
        response.full_name().ends_with(".response"),
        "response resolved to {:?}",
        response.full_name()
    );
    Ok(())
}
