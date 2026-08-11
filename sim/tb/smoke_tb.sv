// Smoke testbench for the TinyALU DUT.
//
// Purpose: prove the simulator toolchain works — compile, elaborate, run,
// self-check. This is NOT the book's testbench; rustdv will drive the DUT
// from Rust. Keep this minimal and portable across simulators.
//
// The harness greps stdout for "SMOKE: PASS" — do not remove that print.

`timescale 1ns/1ns

module smoke_tb;

   logic        reset_n;
   logic        start;
   logic [7:0]  A, B;
   logic [2:0]  op;
   wire         done;
   wire [15:0]  result;
   wire         clk;

   int          errors = 0;

   tinyalu dut (.A, .B, .op, .reset_n, .start, .clk, .done, .result);

   // The DUT supplies its own clock (D112), so this testbench watches it
   // rather than driving one. Nothing here may drive `clk`: two sources on
   // one signal is the bug this smoke test exists to rule out.
   // Hold start and operands until done asserts, then release.
   task do_op(input [7:0] a, input [7:0] b, input [2:0] o,
              input [15:0] expected);
      @(negedge clk);
      A = a; B = b; op = o; start = 1;
      @(negedge clk);
      while (!done) @(negedge clk);
      if (result !== expected) begin
         $display("SMOKE: MISMATCH op=%0d A=%0d B=%0d result=%0d expected=%0d",
                  o, a, b, result, expected);
         errors++;
      end
      start = 0;
      @(negedge clk);
   endtask

   initial begin
      reset_n = 0; start = 0; A = 0; B = 0; op = 0;
      repeat (3) @(negedge clk);
      reset_n = 1;
      repeat (2) @(negedge clk);

      do_op(8'd2,   8'd3,   3'b001, 16'd5);      // ADD
      do_op(8'hF0,  8'h3C,  3'b010, 16'h0030);   // AND
      do_op(8'hF0,  8'h3C,  3'b011, 16'h00CC);   // XOR
      do_op(8'd4,   8'd5,   3'b100, 16'd20);     // MUL (three_cycle)
      do_op(8'hFF,  8'hFF,  3'b001, 16'h01FE);   // ADD carries into bit 8

      if (errors == 0) $display("SMOKE: PASS");
      else             $display("SMOKE: FAIL (%0d mismatches)", errors);
      $finish;
   end

   // Watchdog: a hung handshake must not hang CI.
   initial begin
      #100000;
      $display("SMOKE: FAIL (timeout)");
      $finish;
   end

endmodule
