//test
pragma circom 2.0.6;

include "./Skate.circom";
include "./hashes.circom";

template rollup(n, log_n, m, log_m) {
	assert(2**log_n >= n);
	assert(2**(log_n-1) < n);
	assert(2**log_m >= m);
	assert(2**(log_m-1) < m);

	signal input robots_id[n];						//robot list
	signal input old_robots_tasks_id[n][2];
	signal input old_robots_positions[n][2][3];

	signal input old_robots_root;						//Merkle root of the previous position of the robots

	signal input tasks_id[m];						//task list
	signal input tasks_positions[m][3];

	signal input tasks_root;							//Merkle root of the tasks

	signal output new_robots_root;



	//Verify the robots' Merkle root using Anemoi

	component old_robot_tree = robot_root(n, log_n, m, log_m);
	for(var i=0; i<n; i++){
		old_robot_tree.robot_id[i] <== robots_id[i];
	 	old_robot_tree.tasks_id[i][0] <== old_robots_tasks_id[i][0];
	 	old_robot_tree.tasks_id[i][1] <== old_robots_tasks_id[i][1];
	 	for(var j=0; j<3; j++){
	 		old_robot_tree.positions[i][0][j] <== old_robots_positions[i][0][j];
	 		old_robot_tree.positions[i][1][j] <== old_robots_positions[i][1][j];
	 	}
	}
	log(old_robot_tree.root);
	old_robot_tree.root === old_robots_root;




	//Verify the tasks' Merkle root using Anemoi

	component task_tree = task_root(m, log_m);
	for(var i=0; i<m; i++){
		task_tree.id[i] <== tasks_id[i];
	 	for(var j=0; j<3; j++){
	 		task_tree.position[i][j] <== tasks_positions[i][j];
	 	}
	}
	log(task_tree.root);
	task_tree.root === tasks_root;



	//Update the robots position using the SKATE algorithm 

	component skate = Skate(n, log_n, m, log_m);
	for(var i=0; i<n; i++){
		skate.robots_id[i] <== robots_id[i];
		skate.old_robots_tasks_id[i][0] <== old_robots_tasks_id[i][0];
		skate.old_robots_tasks_id[i][1] <== old_robots_tasks_id[i][1];
		for(var j=0; j<3; j++){
	 		skate.old_robots_positions[i][0][j] <==  old_robots_positions[i][0][j];
	 		skate.old_robots_positions[i][1][j] <==  old_robots_positions[i][1][j];
	 	}
	}
	for(var i=0; i<m; i++){
		skate.tasks_id[i] <== tasks_id[i];
	 	for(var j=0; j<3; j++){
	 		skate.tasks_positions[i][j] <== tasks_positions[i][j];
	 	}
	}



	//Compute the new robot root

	component new_robot_tree = robot_root(n, log_n, m, log_m);
	for(var i=0; i<n; i++){
		new_robot_tree.robot_id[i] <== robots_id[i];
	 	new_robot_tree.tasks_id[i][0] <== skate.new_robots_tasks_id[i][0];
	 	new_robot_tree.tasks_id[i][1] <== skate.new_robots_tasks_id[i][1];
	 	for(var j=0; j<3; j++){
	 		new_robot_tree.positions[i][0][j] <== skate.new_robots_positions[i][0][j];
	 		new_robot_tree.positions[i][1][j] <== skate.new_robots_positions[i][1][j];
	 	}
	}
	new_robots_root <== new_robot_tree.root;
	log(new_robots_root);
}


component main {public [old_robots_root, tasks_root]} = rollup(3,2,3,2);			//Rollup(n) is a setup with n robots and m tasks per circuit