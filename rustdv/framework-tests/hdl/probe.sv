// A design that exists to be measured, not to compute anything.
//
// The book's examples run against the TinyALU because a reader wants a device
// that does something. These tests want the opposite: signals of known widths
// with nothing driving them, so a value read back is the value the framework
// wrote and nothing else. The one exception is `counted`, which the RTL drives
// on every rising edge — an edge trigger needs something to be edging.
`timescale 1ns/1ns
module probe;

   // Driven by the framework (Clock::new(&clk, ..).start()).
   bit clk;

   // Written and read by the framework; nothing in here touches them, so a
   // read-back that differs from the write is the framework's doing.
   logic [7:0]  byte_sig;
   logic [15:0] word_sig;
   logic [3:0]  nibble;
   logic        flag;

   // Combinational path used to prove that a VPI write in ReadWrite is
   // re-evaluated before ReadOnly callbacks observe the design.
   logic [7:0]  comb_in;
   wire [7:0]   comb_out = comb_in ^ 8'hA5;

   // An RTL-owned delayed event used to prove the Verilator host advances to
   // the earlier of an RTL time slot and a pending VPI deadline.
   logic        rtl_event_request;
   logic        rtl_event_done;
   initial rtl_event_done = 0;
   always @(posedge rtl_event_request) #7 rtl_event_done = 1;

   // Never assigned anywhere: stays X for the whole simulation, which is what
   // the X/Z tests read.
   logic        never_driven;
   logic [7:0]  never_driven_bus;

   // Driven by the design, so the framework can watch a signal change without
   // having changed it.
   logic [7:0]  counted;
   initial counted = 0;
   always @(posedge clk) counted <= counted + 1;

   // Icarus elides a variable nothing reads, and an elided variable is not in
   // the VPI namespace — the framework's `signal("byte_sig")` comes back
   // "no object named 'byte_sig' in scope 'probe'". Reading them all into one
   // wire nobody uses keeps them alive without driving them, which is the
   // whole point of declaring them.
   wire keep_alive = ^{byte_sig, word_sig, nibble, flag, comb_in, comb_out,
                       rtl_event_request, rtl_event_done,
                       never_driven, never_driven_bus};

   final $display("RTL FINAL: PASS");

endmodule
