// A compilation-unit class makes Verilator expose a synthetic *_Vclpkg
// scope alongside the requested top module.  rustdv must not mistake that
// implementation detail for the DUT.
`timescale 1ns/1ns

class class_top_transaction;
   bit [7:0] payload;
   static bit class_flag;
endclass

module class_top(
   input  logic [7:0] data_i,
   output logic [7:0] data_o
);
   initial class_top_transaction::class_flag = 0;
   assign data_o = data_i ^ {8{class_top_transaction::class_flag}};

   final $display("RTL FINAL: PASS");
endmodule
