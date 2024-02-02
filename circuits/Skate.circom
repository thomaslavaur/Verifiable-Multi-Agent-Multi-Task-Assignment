//test
pragma circom 2.0.6;

include "../circomlib-master/circuits/comparators.circom";
include "../circomlib-master/circuits/gates.circom";


template distance() {
    signal input x1;
    signal input y1;
    signal input z1;

    signal input x2;
    signal input y2;
    signal input z2;

    signal output out;

    signal sum1;
    signal sum2;
    signal sum3;

    sum1 <== (x1-x2)**2;
    sum2 <== (y1-y2)**2;
    sum3 <== (z1-z2)**2;

    out <== sum1 + sum2 + sum3;
}

template cost_matrix(n,m) {					// n robots and m tasks
	signal input robots_positions[n][3];
	signal input tasks_positions[m][3];
	signal output cost[n][m];

	component d[n][m];
	for(var i=0; i<n;i++){
		for(var j=0;j<m;j++){
			d[i][j] = distance();
			d[i][j].x1 <== robots_positions[i][0];
			d[i][j].y1 <== robots_positions[i][1];
			d[i][j].z1 <== robots_positions[i][2];
			d[i][j].x2 <== tasks_positions[j][0];
			d[i][j].y2 <== tasks_positions[j][1];
			d[i][j].z2 <== tasks_positions[j][2];
			cost[i][j] <== d[i][j].out;
		}
	}
}


function min(array,n) {
	var min = array[0];
	for(var i=0; i<n; i++){
		if(array[i] < min) {
			min = array[i];
		}
	}

	return min;
}


function index(array,min,n) {
	var idx = n;				// Since tasks are less than n; we will return n in case of no match of id (on n bits)
	for(var i=0;i<n;i++){
		if(array[i] == min){
			idx = i;
		}
	}

	return idx;
}


function sort(cost,n,m){
	var length_n = 3;				// ENSURE THAT LENGTH IS EQUAL TO n BECAUSE CIRCOM CANNOT CREATE VAR OF VAR LENGTH (event if known at compilation time)
	var length_m = 3;
	assert(length_n == n);
	assert(length_m == m);
	var idx[length_n][length_m];
	var dist[length_n][length_m];
	var tmp;

	for(var i = 0; i<n; i++){
		for(var j = 0; j<m; j++){
			idx[i][j] = i;
			dist[i][j] = cost[i][j];
		}
	}

	for(var j =0; j<m; j++){
		for(var i = 0; i<n; i++){
			for(var k = i+1; k<n; k++){
				if(dist[k][j] < dist[i][j]){
					tmp = dist[i][j];
                	dist[i][j] = dist[k][j];
                	dist[k][j] = tmp;
                	tmp = idx[i][j];
                	idx[i][j] = idx[k][j];
                	idx[k][j] = tmp;
				}
			}
		}
	}
	return(idx);   							// Return the sorted indexes of the cost             
}

template CalculateTotal(n) {
    signal input in[n];
    signal output out;

    signal sums[n];
    sums[0] <== in[0];

    for (var i=1; i < n; i++) {
        sums[i] <== sums[i - 1] + in[i];
    }

    out <== sums[n - 1];
}


template QuinSelector(n,idx_bits) {		// out = in[index]
    signal input in[n];
    signal input index;
    signal output out;
    
    // Ensure that index < n
    assert(n < 2**idx_bits);
    component lessThan = LessThan(idx_bits);							//Assert less than
    lessThan.in[0] <== index;
    lessThan.in[1] <== n;
    lessThan.out === 1;

    component calcTotal = CalculateTotal(n);
    component eqs[n];

    // For each item, check whether its index equals the input index.
    for (var i = 0; i < n; i ++) {
        eqs[i] = IsEqual();
        eqs[i].in[0] <== i;
        eqs[i].in[1] <== index;

        // eqs[i].out is 1 if the index matches. As such, at most one input to
        // calcTotal is not 0.
        calcTotal.in[i] <== eqs[i].out * in[i];
    }

    // Returns 0 + 0 + ... + item
    out <== calcTotal.out;
}

template verify_permutation(n){
	// We verify that each value between 0 and n-1 appear exactly once.

	signal input in[n];
	signal output out;

	signal test[n];
	component select[n][n];
	for(var i=0; i<n; i++){
		test[i] <== 0;
		for(var j=0; j<n; j++){
			select[i][j] = IsEqual();
		}
	}

	for(var i=0; i<n; i++){
		for(var j=0; j<n; j++){
			select[i][j].in[0] <== i;
			select[i][j].in[1] <== in[j];
		}
	}	

	component sum[n];
	for(var i=0; i<n; i++){
		sum[i] = CalculateTotal(n);
		for(var j=0;j<n;j++){
			sum[i].in[j] <== select[i][j].out;
		}
		1 === sum[i].out;
	}
}


template verify_sorting(n, log_n){
	signal input cost[n];
	signal input sorted_indexes[n];

	component select[n];
	component less[n-1];
	select[0] = QuinSelector(n,log_n+1);
	for(var i=0;i<n-1;i++){
		select[i+1] = QuinSelector(n,log_n+1);
		less[i] = LessEqThan(29);				// Since positions are between 0 and 10 000; costs are between 0 and 3 * 10000**2 = 300 000 000 so on 29 bits
	}

	for(var i=0; i<n; i++){
		for(var j=0; j<n; j++){
			select[i].in[j] <== cost[j];
		}
		select[i].index <== sorted_indexes[i];
	}
	for(var i=0; i<n-1;i++){
		less[i].in[0] <== select[i].out;
		less[i].in[1] <== select[i+1].out;
		less[i].out === 1;
	}
}


template rank_matrix(n, log_n, m) {
	signal input cost[n][m];		
	signal output rank[n][m];

	var idx[n][m] = sort(cost,n,m);
	for(var i=0; i<n; i++){
		for(var j=0; j<m; j++){
			rank[i][j] <-- idx[i][j];
		}
	}

	// Here we need to ensure that 1) idx is the correct sort of cost indexes and 2) it really is a permutation of those indexes


	// 1) Verify that it's correctly sorting
	component verify_s[m];
	for(var collumn=0; collumn<m; collumn++){
		verify_s[collumn] = verify_sorting(n, log_n);
		for(var l=0;l<n;l++){
			verify_s[collumn].cost[l] <== cost[l][collumn];
			verify_s[collumn].sorted_indexes[l] <== rank[l][collumn];
		}
	}

	// 2) Verify that it's a permutation:
	component verify_p[m];
	for(var collumn=0; collumn<m; collumn++){
		verify_p[collumn] = verify_permutation(n);
		for(var l=0;l<n;l++){
			verify_p[collumn].in[l] <== rank[l][collumn];
		}
	}
}


template task_choice(m,log_m) {
	signal input ranks[m];
	signal input available[m];
	signal input robot_id;
	signal input costs[m];				// Since costs are less than or equal to 300 000 000; we will use 399 999 999 wich also is on 29 bits to check if id match
	signal output task_id;

	var array[m];
	for(var i=0; i<m; i++){
		if(ranks[i] == robot_id && available[i] == 1) {
			array[i] = costs[i];
		} else {
			array[i] = 399999999;
		}
	}

	var min = min(array,m);

	signal mini;
	mini <-- min;
	task_id <-- index(costs,min,m);

	// Verify that costs[i] is less than all other cost matching the id or that the id doesn't match

	component eq[m];
	component less[m];
	signal take[m];
	for(var i=0; i<m; i++){
		eq[i] = IsEqual();
		eq[i].in[0] <== ranks[i];
		eq[i].in[1] <== robot_id;
		take[i] <== eq[i].out * available[i];

		less[i] = LessEqThan(29);
		less[i].in[0] <== mini;
		less[i].in[1] <== take[i] * costs[i] +(1-take[i]) * 399999999;
		less[i].out === 1;
	}


	// Verify that the task id match the minimal cost

	component equal = IsEqual();
	equal.in[0] <== m;
	equal.in[1] <== task_id;
	component select = QuinSelector(m,log_m+1);
	for(var i=0;i<m;i++){
		select.in[i] <== costs[i];
	}
	select.index <== (1 - equal.out) * task_id;			// We dont care about what we select if they are no minimum matching the id
	signal intermediate_value;
	intermediate_value <== (1 - equal.out) * mini;
	select.out === intermediate_value + equal.out * costs[0];
}


template assign(n,m, log_m) {
	signal input ranks[n][m];
	signal input costs[n][m];
	signal output assign[n];

	signal sum[n][n][2];

	signal robot_available[n][n];
	signal task_available[n][m];
	for(var i=0;i<n;i++){
		robot_available[0][i] <== 1;		// Every robot and task are supposed available at the begining
	}
	for(var i=0;i<m;i++){
		task_available[0][i] <== 1;
	}

	component task_choice[n][n];
	component eq[n][n];
	component eq2[n][n][m];
	component sums[n-1][m];


	for(var i=0;i<n;i++){		// for each line
		for(var j=0;j<n;j++){		// for each robot
			task_choice[i][j] = task_choice(m, log_m);
			eq[i][j] = IsEqual();


			task_choice[i][j].robot_id <== j;
			for(var k=0;k<m;k++){		//for each task
				task_choice[i][j].ranks[k] <== ranks[i][k];
				task_choice[i][j].costs[k] <== costs[i][k];
				task_choice[i][j].available[k] <== task_available[i][k];
			}
			eq[i][j].in[0] <== m;
			eq[i][j].in[1] <== task_choice[i][j].task_id;

			sum[j][i][0] <== (1 - eq[i][j].out) * robot_available[i][j];
			sum[j][i][1] <== sum[j][i][0] * task_choice[i][j].task_id;
			if(i != n-1) {
				robot_available[i+1][j] <== robot_available[i][j] * eq[i][j].out;
			}
			for(var k=0; k<m; k++){		//for each task
				eq2[i][j][k] = IsEqual();
				eq2[i][j][k].in[0] <== k;
				eq2[i][j][k].in[1] <== task_choice[i][j].task_id;
			}
		}
		if(i!=n-1){
			for(var j=0; j<m; j++){		//for each task
				sums[i][j] = CalculateTotal(n);
				for(var k=0; k<n; k++){		//for each robot
					sums[i][j].in[k] <== eq2[i][k][j].out * robot_available[i][k];
				}
				task_available[i+1][j] <== (1 - sums[i][j].out) * task_available[i][j];
			}
		}
	}


	component sums2[n];
	for(var i=0; i<n; i++){		// for each robot
		sums2[i] = CalculateTotal(n);
		for(var j=0;j<n;j++){		//for each round
			sums2[i].in[j] <== sum[i][j][1];
		}
		assign[i] <== sums2[i].out;
	}
}


template Skate(n, log_n, m, log_m) {
	signal input robots_id[n];
	signal input old_robots_tasks_id[n][2];
	signal input old_robots_positions[n][2][3];

	signal input tasks_id[m];
	signal input tasks_positions[m][3];

	signal output new_robots_tasks_id[n][2];
	signal output new_robots_positions[n][2][3];

	component cost = cost_matrix(n,m);
	for(var i=0; i<n; i++){
		for(var j=0; j<3; j++){
			cost.robots_positions[i][j] <== old_robots_positions[i][1][j];
		}
	}
	for(var i=0; i<m; i++){
		for(var j=0; j<3; j++){
			cost.tasks_positions[i][j] <== tasks_positions[i][j];
		}
	}


	component rank = rank_matrix(n, log_n, m);
	for(var i=0; i<n; i++){
		for(var j=0; j<m; j++){
			rank.cost[i][j] <== cost.cost[i][j];
		}
	}

	component assign = assign(n,m, log_m);
	for(var i=0; i<n; i++){
		for(var j=0; j<m; j++){
			assign.ranks[i][j] <== rank.rank[i][j];
			assign.costs[i][j] <== cost.cost[i][j];
		}
	}


	component select_pos[n][3];
	component select_id[n];
	for(var i=0; i<n; i++){
		log(assign.assign[i]);
		select_id[i] = QuinSelector(m,log_m+1);
		select_id[i].index <== assign.assign[i];
		for(var j=0; j<m; j++){
			select_id[i].in[j] <== tasks_id[j];
		}
		new_robots_tasks_id[i][0] <== old_robots_tasks_id[i][1];
		new_robots_tasks_id[i][1] <== select_id[i].out;


		for(var j=0; j<3; j++){
			select_pos[i][j] = QuinSelector(m,log_m+1);
			select_pos[i][j].index <== new_robots_tasks_id[i][1];
			for(var k=0; k<m; k++){
				select_pos[i][j].in[k] <== tasks_positions[k][j];
			}
			new_robots_positions[i][0][j] <== old_robots_positions[i][1][j];
			new_robots_positions[i][1][j] <== select_pos[i][j].out;
		}
	}
}



//component main {public [hashRobots, hashTasks, blockNumber]} = final(250,30,100,2);