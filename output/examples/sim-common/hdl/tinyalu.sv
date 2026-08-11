module tinyalu (input [7:0] A,
		input [7:0] B,
		input [2:0] op,
		input reset_n,
		input start,
		output bit clk,
		output done,
		output [15:0] result);

   // The RTL supplies its own clock, as it does in both earlier books and as
   // it must on an emulator, where a software-driven clock would cross the
   // hardware boundary on every edge. rustdv generates no clock; BFMs only
   // ever wait on edges. 10ns period at the 1ns/1ns timescale.
   initial clk = 0;
   always #5 clk = ~clk;

   wire [15:0] 		      result_aax, result_mult;
   wire 		          start_single, start_mult;
   wire                   done_aax;
   wire                   done_mult;

   assign start_single = start & ~op[2];
   assign start_mult   = start & op[2];

   single_cycle and_add_xor (.A, .B, .op, .clk, .reset_n, .start(start_single),
			     .done(done_aax), .result(result_aax));

   three_cycle mult (.A, .B, .op, .clk, .reset_n, .start(start_mult),
		    .done(done_mult), .result(result_mult));


   assign done = (op[2]) ? done_mult : done_aax;

   assign result = (op[2]) ? result_mult : result_aax;

endmodule // tinyalu


module single_cycle(input [7:0] A,
		   input [7:0] B,
		   input [2:0] op,
		   input clk,
		   input reset_n,
		   input start,
		   output logic done,
		   output logic [15:0] result);

  always @(posedge clk)
    if (!reset_n)
      result <= 0;
    else
      case(op)
		3'b001 : result <= {8'd0,A} + {8'd0,B};
		3'b010 : result <= {8'd0,A} & {8'd0,B};
		3'b011 : result <= {8'd0,A} ^ {8'd0,B};
		default : result <= {A,B};
      endcase // case (op)

   always @(posedge clk)
     if (!reset_n)
       done <= 0;
     else
       done <= ((start == 1'b1) && (op != 3'b000));

endmodule : single_cycle


module three_cycle(input [7:0] A,
		   input [7:0] B,
		   input [2:0] op,
		   input clk,
		   input reset_n,
		   input start,
		   output logic done,
		   output logic [15:0] result);

   logic [7:0] 			       a_int, b_int;
   logic [15:0] 		       mult1, mult2;
   logic 			       done1, done2, done3;

   always @(posedge clk)
     if (!reset_n) begin
	done  <= 0;
	done3 <= 0;
	done2 <= 0;
	done1 <= 0;
	a_int <= 0;
	b_int <= 0;
	mult1 <= 0;
	mult2 <= 0;
	result<= 0;
     end else begin // if (!reset_n)
	a_int  <= A;
	b_int  <= B;
	mult1  <= a_int * b_int;
	mult2  <= mult1;
	result <= mult2;
	done3  <= start & !done;
	done2  <= done3 & !done;
	done1  <= done2 & !done;
	done   <= done1 & !done;
     end // else: !if(!reset_n)
endmodule : three_cycle
