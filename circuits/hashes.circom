//test
pragma circom 2.0.6;

include "./anemoi_2_to_1_Baby_Jubjub.circom";
include "./anemoi_4_to_1_Baby_Jubjub.circom";
include "../circomlib-master/circuits/comparators.circom";
include "../circomlib-master/circuits/switcher.circom";

/* A task is:
	- 1 id (enforced by merkle position)
	- 3 positions [x,y,z]
*/

template hash_task(m, log_m){
	signal input id;
	signal input position[3];
	signal output out;				

	component less[4];
	less[0] = LessThan(log_m); 		// There is a maximum of 1023 tasks
	less[0].in[0] <== id;
	less[0].in[1] <== m;
	less[1] = LessThan(14);		// Positions are between 0 and 10 000 so on 14 bits
	less[1].in[0] <== position[0];
	less[1].in[1] <== 10000;
	less[2] = LessThan(14);
	less[2].in[0] <== position[1];
	less[2].in[1] <== 10000;
	less[3] = LessThan(14);
	less[3].in[0] <== position[2];
	less[3].in[1] <== 10000;

	less[0].out + less[1].out + less[2].out + less[3].out === 4; // Check inequalities hold


	component hash = hash_2_to_1();
	hash.in[0] <== id * 2**14 + position[0];
	hash.in[1] <== position[1] * 2**14 + position[2];
	out <== hash.out;
}


/* A Robot is:
	- 1 id
	- 1 distance before end
	- n tasks (its position being the last task posiion)
*/

template hash_robot(n,m, log_m){				// We suppose that we have 2 tasks per robots
	signal input robot_id;
	signal input tasks_id[2];
	signal input positions[2][3];
	signal output out;

	component h_task[2];
	for(var i=0; i<2; i++){
		h_task[i] = hash_task(m, log_m);
		h_task[i].id <== tasks_id[i];
		h_task[i].position[0] <== positions[i][0];
		h_task[i].position[1] <== positions[i][1];
		h_task[i].position[2] <== positions[i][2];
	}

	component hash = hash_4_to_1();
	hash.in[0] <== robot_id;
	hash.in[1] <== h_task[0].out;
	hash.in[2] <== h_task[1].out;
	hash.in[3] <== 0;
	out <== hash.out;
}



template merkle_tree(n){
	signal input nodes[2**n]; //suposed in order
	signal output root;

	assert(n >= 2);		//if n = 1, use hash_2_to_1 template

	component h[n][2**(n-1)];
	for(var i=0; i<n; i++){
		for(var j=0; j<2**(n-i-1);j++){
			h[i][j] = hash_2_to_1();
			if(i == 0){
				h[i][j].in[0] <== nodes[2*j];
				h[i][j].in[1] <== nodes[2*j+1];
			} else {
				h[i][j].in[0] <== h[i-1][2*j].out;
				h[i][j].in[1] <== h[i-1][2*j+1].out;
			}
		}
	}

	root <== h[n-1][0].out;
}



template task_root(m,log_m) {				//We have m tasks
	signal input id[m];
	signal input position[m][3];
	signal output root;

	component h_tasks[m];
	for(var i = 0; i<m; i++){
		h_tasks[i] = hash_task(m, log_m);
		h_tasks[i].id <== id[i];
		h_tasks[i].position[0] <== position[i][0];
		h_tasks[i].position[1] <== position[i][1];
		h_tasks[i].position[2] <== position[i][2];
	}

	component tree = merkle_tree(log_m);
	for(var i=0; i<2**log_m; i++){
		if(i < m) {
			tree.nodes[i] <== h_tasks[i].out;
		} else {
			tree.nodes[i] <== 103860425244306721054357604449078699979184018657001128167783972180760304967; //hash of a task will all values 0
		}
	}

	root <== tree.root;
}


template robot_root(n,log_n,m, log_m) {			// We have n robots and m tasks
	signal input robot_id[n];
	signal input tasks_id[n][2];
	signal input positions[n][2][3];
	signal output root;

	component h_robot[n];
	for(var i=0; i<n; i++){
		h_robot[i] = hash_robot(n,m, log_m);
		h_robot[i].robot_id <== robot_id[i];
		h_robot[i].tasks_id[0] <== tasks_id[i][0];
		h_robot[i].tasks_id[1] <== tasks_id[i][1];
		for(var j=0; j<2; j++){
			for(var k=0; k<3; k++){
				h_robot[i].positions[j][k] <== positions[i][j][k];
			}
		}
	}

	component tree = merkle_tree(log_n);
	for(var i=0; i<2**log_n; i++){
		if(i < n) {
			tree.nodes[i] <== h_robot[i].out;
		} else {
			tree.nodes[i] <== 19803829510264496905782185690924016388609305741426681378119315514000584486177; //hash of a robot will all values 0
		}
	}

	root <== tree.root;
}