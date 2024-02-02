//test
pragma circom 2.0.6;

include "anemoi_Baby_Jubjub_4_to_1_constants.circom";

template ark_layer_4_to_1(round_number) {
	assert(round_number < 14);

	signal input in[4];
	signal output out[4];

	var C[14][2] = C_4_to_1();
	var D[14][2] = D_4_to_1();

	out[0] <== in[0] + C[round_number][0];
	out[1] <== in[1] + C[round_number][1];
	out[2] <== in[2] + D[round_number][0];
	out[3] <== in[3] + D[round_number][1];
}

template pow_alpha_4_to_1() { // ALPHA = 5
	signal input in;
	signal output out;

	signal in2;
	signal in4;

	in2 <== in*in;
	in4 <== in2 * in2;
	out <== in4 * in;
}

template mds_layer_4_to_1() {
	signal input in[4];
	signal output out[4];

	signal x0;
	signal x1;
	signal y0;
	signal y1;

	x0 <== in[0] + 5*in[1];
	x1 <== 5*in[0] + 26*in[1]; 

	y0 <== in[3] + 5*in[2];
	y1 <== 5*in[3] + 26*in[2];

	out[2] <== y0 + x0;
	out[3] <== y1 + x1;
	out[0] <== x0 + out[2];
	out[1] <== x1 + out[3]; 
}


template s_box_4_to_1() {
	signal input in[4];
	signal output out[4];


	//Calculation using open Flystel
	var x0;
	var y0;
	x0 = in[0];
	y0 = in[2];

	x0 = x0 - 5 * (y0**2);
	y0 = y0 - (x0**17510594297471420177797124596205820070838691520332827474958563349260646796493); //   1/ALPHA
	x0 = x0 + 5 * (y0**2) + 8755297148735710088898562298102910035419345760166413737479281674630323398247; //DELTA

	var x1;
	var y1;
	x1 = in[1];
	y1 = in[3];

	x1 = x1 - 5 * (y1**2);
	y1 = y1 - (x1**17510594297471420177797124596205820070838691520332827474958563349260646796493); //   1/ALPHA
	x1 = x1 + 5 * (y1**2) + 8755297148735710088898562298102910035419345760166413737479281674630323398247; //DELTA

	out[0] <-- x0;
	out[1] <-- x1;
	out[2] <-- y0;
	out[3] <-- y1;


	//Verification using closed Flystel

	component pow[2];
	pow[0] = pow_alpha_4_to_1();
	pow[1] = pow_alpha_4_to_1();


	pow[0].in <== in[2] - out[2];
	signal y0_2;
	signal v0_2;
	y0_2 <== in[2]*in[2];
	v0_2 <==out[2]*out[2];
	in[0] === pow[0].out + 5 * y0_2;
	out[0] === pow[0].out + 5 * v0_2 + 8755297148735710088898562298102910035419345760166413737479281674630323398247;// DELTA

	pow[1].in <== in[3] - out[3];
	signal y1_2;
	signal v1_2;
	y1_2 <== in[3]*in[3];
	v1_2 <==out[3]*out[3];
	in[1] === pow[1].out + 5 * y1_2;
	out[1] === pow[1].out + 5 * v1_2 + 8755297148735710088898562298102910035419345760166413737479281674630323398247;// DELTA
}

template round_4_to_1(round_number) {
	signal input in[4];
	signal output out[4];

	component cst = ark_layer_4_to_1(round_number);
	component mds = mds_layer_4_to_1();
	component sbox = s_box_4_to_1();

	cst.in[0] <== in[0];
	cst.in[1] <== in[1];
	cst.in[2] <== in[2];
	cst.in[3] <== in[3];
	mds.in[0] <== cst.out[0];
	mds.in[1] <== cst.out[1];
	mds.in[2] <== cst.out[2];
	mds.in[3] <== cst.out[3];
	sbox.in[0] <== mds.out[0];
	sbox.in[1] <== mds.out[1];
	sbox.in[2] <== mds.out[2];
	sbox.in[3] <== mds.out[3];
	out[0] <== sbox.out[0];
	out[1] <== sbox.out[1];
	out[2] <== sbox.out[2];
	out[3] <== sbox.out[3];
}

template permutation_4_to_1(){
	signal input in[4];
	signal output out[4];

	component rounds[14];
	component mds = mds_layer_4_to_1();

	for(var i = 0; i<14; i++){	//14 rounds 
		rounds[i] = round_4_to_1(i);
		if(i==0){
			rounds[i].in[0] <== in[0];
			rounds[i].in[1] <== in[1];
			rounds[i].in[2] <== in[2];
			rounds[i].in[3] <== in[3];
		} else {
			rounds[i].in[0] <== rounds[i-1].out[0];
			rounds[i].in[1] <== rounds[i-1].out[1];
			rounds[i].in[2] <== rounds[i-1].out[2];
			rounds[i].in[3] <== rounds[i-1].out[3];
		}
	}
	mds.in[0] <== rounds[13].out[0];
	mds.in[1] <== rounds[13].out[1];
	mds.in[2] <== rounds[13].out[2];
	mds.in[3] <== rounds[13].out[3];
	out[0] <== mds.out[0];
	out[1] <== mds.out[1];
	out[2] <== mds.out[2];
	out[3] <== mds.out[3];
}

template hash_4_to_1(){
	signal input in[4];
	signal output out;

	component perm = permutation_4_to_1();
	perm.in[0] <== in[0];
	perm.in[1] <== in[1];
	perm.in[2] <== in[2];
	perm.in[3] <== in[3];

	out <== in[0] + perm.out[0] + in[1] + perm.out[1] + in[2] + perm.out[2] + in[3] + perm.out[3];
}

//component main = hash();