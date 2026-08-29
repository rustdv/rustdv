// Structs used directly as top-level ports are vpiStructVar objects in the
// simulator. rustdv must make those named port objects discoverable through
// its ordinary LogicHandle API.
`timescale 1ns/1ns

typedef struct {
   logic       valid;
   logic [7:0] payload;
} struct_port_t;

module struct_ports_top(
   input  struct_port_t request,
   output struct_port_t response
);
   assign response = request;

   final $display("RTL FINAL: PASS");
endmodule
