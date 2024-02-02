use crate::anemoi_2_to_1::hash_2_to_1;
use crate::anemoi_4_to_1::hash_4_to_1;
use crate::Skate::{Robot, Task};
use ark_bn254::Fr;
use ark_ff::PrimeField;
use ark_std::iterable::Iterable;

fn hash_task(task: Task) -> Fr {
    hash_2_to_1(
        task.task_id * Fr::from(2i32.pow(14)) + task.x,
        task.y * Fr::from(2i32.pow(14)) + task.z,
    )
}

fn hash_robot(robot: Robot) -> Fr {
    let hash = [
        hash_task(robot.list_tasks[0]),
        hash_task(robot.list_tasks[1]),
    ];
    hash_4_to_1(robot.robot_id, hash[0], hash[1], Fr::from(0))
}

/*fn merkle_tree(nodes: [Fr; 2usize.pow(N as u32)]) -> Fr {
    assert!(N >= 2);

    let mut h = [[Fr::from(0); 2usize.pow((N - 1) as u32)]; N];
    for i in 0..N {
        for j in 0..2usize.pow((N - i - 1) as u32) {
            if i == 0 {
                h[i][j] = hash_2_to_1(nodes[2 * j], nodes[2 * j + 1]);
            } else {
                h[i][j] = hash_2_to_1(h[i - 1][2 * j], h[i - 1][2 * j + 1]);
            }
        }
    }

    h[N - 1][0]
}*/

fn merkle_tree(nodes: Vec<Fr>) -> Vec<Fr> {
    if nodes.len() == 2 {
        vec![hash_2_to_1(nodes[0], nodes[1])]
    } else {
        merkle_tree(
            (0..nodes.len() / 2)
                .map(|i| hash_2_to_1(nodes[2 * i], nodes[2 * i + 1]))
                .collect(),
        )
    }
}

fn merkle_tree_with_proof(nodes: Vec<Fr>, proofs: &mut Vec<Vec<Fr>>, level: usize) -> Fr {
    if nodes.len() == 2 {
        let h = hash_2_to_1(nodes[0], nodes[1]);
        h
    } else {
        let hashes: Vec<Fr> = (0..nodes.len() / 2)
            .map(|i| hash_2_to_1(nodes[2 * i], nodes[2 * i + 1]))
            .collect();
        for h in hashes.clone() {
            proofs[level + 1].push(h);
        }
        merkle_tree_with_proof(hashes, proofs, level + 1)
    }
}

pub fn task_root(tasks: &Vec<Task>) -> Fr {
    let mut first_hash: Vec<Fr> = (0..tasks.len())
        .map(|i| hash_task(tasks[i]))
        .collect();
    while first_hash.len() < 2usize.pow(tasks.len().ilog2() + 1) {
        first_hash.push("103860425244306721054357604449078699979184018657001128167783972180760304967".parse::<Fr>().unwrap());
    }
    merkle_tree(first_hash)[0]
}

pub fn robot_root(robots: &Vec<Robot>) -> (Fr, Vec<Vec<Fr>>) {

    let mut first_hash: Vec<Fr> = (0..robots.len())
        .map(|i| hash_robot(robots[i]))
        .collect();
    while first_hash.len() < 2usize.pow(robots.len().ilog2() + 1) {
        first_hash.push("19803829510264496905782185690924016388609305741426681378119315514000584486177".parse::<Fr>().unwrap());
    }
    let mut proofs = vec![vec![]; robots.len().ilog2() as usize + 1];
    for i in 0..2usize.pow(robots.len().ilog2() + 1) {
        proofs[0].push(first_hash[i]);
    }
    (merkle_tree_with_proof(first_hash, &mut proofs, 0), proofs)
}

pub fn verify_robot_in_tree(robot: Robot, root: Fr, proof: Vec<Fr>) -> bool {
    let mut hash = hash_robot(robot);
    let mut selector;
    for i in 0..proof.len() {
        selector = (robot
            .robot_id
            .into_bigint()
            .to_string()
            .parse::<i32>()
            .unwrap()
            >> i)
            & 1;
        if selector == 1 {
            hash = hash_2_to_1(proof[i], hash);
        } else {
            hash = hash_2_to_1(hash, proof[i]);
        }
    }
    hash == root
}

pub fn get_merkle_proof_from_id(robot_id: Fr, proofs: &Vec<Vec<Fr>>) -> Vec<Fr> {
    let mut proof = vec![Fr::from(0); proofs[0].len().ilog2() as usize];

    for i in 0..proofs[0].len().ilog2() as usize {
        if robot_id.into_bigint().to_string().parse::<usize>().unwrap() >> i & 1 == 1 {
            proof[i] =
                proofs[i][(robot_id.into_bigint().to_string().parse::<usize>().unwrap() >> i) - 1];
        } else {
            proof[i] =
                proofs[i][(robot_id.into_bigint().to_string().parse::<usize>().unwrap() >> i) + 1];
        }
    }
    proof
}
