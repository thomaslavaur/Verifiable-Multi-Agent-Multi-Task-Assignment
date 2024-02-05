// CENTRAL POINT

use ark_bn254::{Bn254, Fr};
use ark_circom::{read_zkey, CircomBuilder, CircomConfig, CircomReduction};
use ark_crypto_primitives::snark::SNARK;
use ark_ff::{Field, PrimeField};
use ark_groth16::{Groth16, Proof};
use ark_std::rand::{thread_rng, Rng};
use num_bigint::BigInt;
use reqwest::Error;
use serde::Deserialize;
use std::fs::File;
use std::path::PathBuf;
use std::time::Duration;
use std::{io, thread};
use SKATE::hashes::{get_merkle_proof_from_id, robot_root, task_root};
use SKATE::Skate::{skate, Robot, Task};

type GrothBn = Groth16<Bn254, CircomReduction>;
#[derive(Debug, Deserialize)]
struct ClientConfig {
    list_ip: Vec<String>,
    x: Vec<String>,
    y: Vec<String>,
    z: Vec<String>,
    clock: u64,
    manual_choices: usize, // The number of manually generation per clock
    iterations: usize,     // Number of loop (0 for manual extinction)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut rng = thread_rng();

    let file_path = std::env::current_dir()
        .unwrap()
        .join(PathBuf::from("config.toml"));

    // Lire le contenu du fichier dans une chaîne
    let toml_content = match std::fs::read_to_string(file_path) {
        Ok(contenu) => contenu,
        Err(e) => {
            eprintln!("Cannot read the configuration file : {}", e);
            std::process::exit(1);
        }
    };

    // Désérialiser la chaîne TOML en une structure Rust
    let configuration: ClientConfig = match toml::from_str(&toml_content) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Cannot deserialize the configuration file : {}", e);
            std::process::exit(1);
        }
    };

    assert_eq!(
        configuration.list_ip.len(),
        3
    );
    assert_eq!(
        configuration.x.len(),
        3
    );
    assert_eq!(
        configuration.y.len(),
        3
    );
    assert_eq!(
        configuration.z.len(),
        3
    );
    assert!(configuration.manual_choices <= 6);
    for i in 0..3 {
        assert!(configuration.x[i].parse::<Fr>().unwrap() <= Fr::from(10000));
        assert!(configuration.y[i].parse::<Fr>().unwrap() <= Fr::from(10000));
        assert!(configuration.z[i].parse::<Fr>().unwrap() <= Fr::from(10000));
    }

    let mut list_robot = vec![];
    for i in 0..3 {
        list_robot.push(Robot {
            robot_id: Fr::from(i as i32),
            list_tasks: [
                Task {
                    task_id: Fr::from(0),
                    x: Fr::from(0),
                    y: Fr::from(0),
                    z: Fr::from(0),
                },
                Task {
                    task_id: Fr::from(1),
                    x: configuration.x[i].parse::<Fr>().unwrap(),
                    y: configuration.y[i].parse::<Fr>().unwrap(),
                    z: configuration.z[i].parse::<Fr>().unwrap(),
                },
            ],
        })
    }
    let mut root = robot_root(&list_robot).0;

    let mut counter: usize = 0;
    while counter < configuration.iterations {
        let mut list_task = vec![];
        for i in 0..configuration.manual_choices {

            let mut user_input_x = String::new();
            let mut user_input_y = String::new();
            let mut user_input_z = String::new();

            println!("Enter the task x coordinates:");
            io::stdin()
                .read_line(&mut user_input_x)
                .expect("Failed to read line");
            println!("Enter the task y coordinates:");
            io::stdin()
                .read_line(&mut user_input_y)
                .expect("Failed to read line");
            println!("Enter the task z coordinates:");
            io::stdin()
                .read_line(&mut user_input_z)
                .expect("Failed to read line");

            let x = user_input_x.trim().parse::<Fr>().unwrap();
            let y = user_input_y.trim().parse::<Fr>().unwrap();
            let z = user_input_z.trim().parse::<Fr>().unwrap();
            if x < Fr::from(10000) && y < Fr::from(10000) && z < Fr::from(10000) {
                list_task.push(Task {
                    task_id: Fr::from(i as u32),
                    x,
                    y,
                    z,
                })
            } else {
                println!("invalid input, a random task will be generated instead.");
                list_task.push(Task {
                    task_id: Fr::from(i as i32),
                    x: Fr::from(rng.gen_range(0..10000)),
                    y: Fr::from(rng.gen_range(0..10000)),
                    z: Fr::from(rng.gen_range(0..10000)),
                })
            }
            println!();
        }

        for i in configuration.manual_choices..6
        {
            list_task.push(Task {
                task_id: Fr::from(i as i32),
                x: Fr::from(rng.gen_range(0..10000)),
                y: Fr::from(rng.gen_range(0..10000)),
                z: Fr::from(rng.gen_range(0..10000)),
            })
        }

        let task_root = task_root(&list_task);
        let list_new_robot = skate(&list_robot, &list_task);
        let (new_robot_root, merkle_proofs) =
            robot_root(&list_new_robot);
        let cfg = CircomConfig::<Bn254>::new(
            std::env::current_dir()
                .unwrap()
                .join("skate_6_3.wasm"),
            std::env::current_dir()
                .unwrap()
                .join("skate_6_3.r1cs"),
        )
        .unwrap();
        let mut builder = CircomBuilder::new(cfg);

        for i in 0..list_robot.len() {
            builder.push_input(
                "robots_id",
                Fr::from(i as i32)
                    .into_bigint()
                    .to_string()
                    .parse::<BigInt>()
                    .unwrap(),
            );
        }
        for i in 0..list_robot.len() {
            builder.push_input(
                "old_robots_tasks_id",
                list_robot[i].list_tasks[0]
                    .task_id
                    .into_bigint()
                    .to_string()
                    .parse::<BigInt>()
                    .unwrap(),
            );
            builder.push_input(
                "old_robots_tasks_id",
                list_robot[i].list_tasks[1]
                    .task_id
                    .into_bigint()
                    .to_string()
                    .parse::<BigInt>()
                    .unwrap(),
            );
        }
        for i in 0..list_robot.len() {
            for k in 0..2 {
                for j in 0..3 {
                    builder.push_input(
                        "old_robots_positions",
                        match j {
                            0i32 => list_robot[i].list_tasks[k].x,
                            1i32 => list_robot[i].list_tasks[k].y,
                            2i32 => list_robot[i].list_tasks[k].z,
                            _ => Fr::from(0),
                        }
                        .into_bigint()
                        .to_string()
                        .parse::<BigInt>()
                        .unwrap(),
                    );
                }
            }
        }
        builder.push_input(
            "old_robots_root",
            root.into_bigint().to_string().parse::<BigInt>().unwrap(),
        );
        for i in 0..list_task.len() {
            builder.push_input(
                "tasks_id",
                list_task[i]
                    .task_id
                    .into_bigint()
                    .to_string()
                    .parse::<BigInt>()
                    .unwrap(),
            );
        }
        for i in 0..list_task.len() {
            for j in 0..3 {
                builder.push_input(
                    "tasks_positions",
                    match j {
                        0 => list_task[i].x,
                        1 => list_task[i].y,
                        2 => list_task[i].z,
                        _ => Fr::from(0),
                    }
                    .into_bigint()
                    .to_string()
                    .parse::<BigInt>()
                    .unwrap(),
                );
            }
        }
        builder.push_input(
            "tasks_root",
            task_root
                .into_bigint()
                .to_string()
                .parse::<BigInt>()
                .unwrap(),
        );

        let params = read_zkey(
            &mut File::open(
                std::env::current_dir()
                    .unwrap()
                    .join("skate_6_3.zkey"),
            )
            .unwrap(),
        )
        .unwrap()
        .0;

        let circom = builder.build().unwrap();

        let zkproof = GrothBn::prove(&params, circom, &mut rng).unwrap();

        for i in 0..list_robot.len() {
            let merkle_proof = get_merkle_proof_from_id(
                Fr::from(i as u32),
                &merkle_proofs,
            );
            let _ = post(
                new_robot_root,
                task_root,
                &zkproof,
                list_new_robot[i],
                merkle_proof,
                &configuration.list_ip[i],
                "6_3"
            )
            .await;
        }

        list_robot = list_new_robot;
        root = new_robot_root;


        for i in 0..list_robot.len() {
            for j in (0..list_task.len()).rev() {
                if list_task[j].task_id == list_robot[i].list_tasks[1].task_id {
                    _ = list_task.remove(j);
                }
            }
        }
        for i in 0..list_task.len() {
            list_task[i].task_id = Fr::from(i as u32);
        }


        let task_root = SKATE::hashes::task_root(&list_task);
        let list_new_robot = skate(&list_robot, &list_task);
        let (new_robot_root, merkle_proofs) =
            robot_root(&list_new_robot);

        let cfg = CircomConfig::<Bn254>::new(
            std::env::current_dir()
                .unwrap()
                .join("skate_3_3.wasm"),
            std::env::current_dir()
                .unwrap()
                .join("skate_3_3.r1cs"),
        )
            .unwrap();
        let mut builder = CircomBuilder::new(cfg);

        for i in 0..list_robot.len() {
            builder.push_input(
                "robots_id",
                Fr::from(i as i32)
                    .into_bigint()
                    .to_string()
                    .parse::<BigInt>()
                    .unwrap(),
            );
        }
        for i in 0..list_robot.len() {
            builder.push_input(
                "old_robots_tasks_id",
                list_robot[i].list_tasks[0]
                    .task_id
                    .into_bigint()
                    .to_string()
                    .parse::<BigInt>()
                    .unwrap(),
            );
            builder.push_input(
                "old_robots_tasks_id",
                list_robot[i].list_tasks[1]
                    .task_id
                    .into_bigint()
                    .to_string()
                    .parse::<BigInt>()
                    .unwrap(),
            );
        }
        for i in 0..list_robot.len() {
            for k in 0..2 {
                for j in 0..3 {
                    builder.push_input(
                        "old_robots_positions",
                        match j {
                            0i32 => list_robot[i].list_tasks[k].x,
                            1i32 => list_robot[i].list_tasks[k].y,
                            2i32 => list_robot[i].list_tasks[k].z,
                            _ => Fr::from(0),
                        }
                            .into_bigint()
                            .to_string()
                            .parse::<BigInt>()
                            .unwrap(),
                    );
                }
            }
        }
        builder.push_input(
            "old_robots_root",
            root.into_bigint().to_string().parse::<BigInt>().unwrap(),
        );
        for i in 0..list_task.len() {
            builder.push_input(
                "tasks_id",
                list_task[i]
                    .task_id
                    .into_bigint()
                    .to_string()
                    .parse::<BigInt>()
                    .unwrap(),
            );
        }
        for i in 0..list_task.len() {
            for j in 0..3 {
                builder.push_input(
                    "tasks_positions",
                    match j {
                        0 => list_task[i].x,
                        1 => list_task[i].y,
                        2 => list_task[i].z,
                        _ => Fr::from(0),
                    }
                        .into_bigint()
                        .to_string()
                        .parse::<BigInt>()
                        .unwrap(),
                );
            }
        }
        builder.push_input(
            "tasks_root",
            task_root
                .into_bigint()
                .to_string()
                .parse::<BigInt>()
                .unwrap(),
        );

        let params = read_zkey(
            &mut File::open(
                std::env::current_dir()
                    .unwrap()
                    .join("skate_3_3.zkey"),
            )
                .unwrap(),
        )
            .unwrap()
            .0;

        let circom = builder.build().unwrap();

        let zkproof = GrothBn::prove(&params, circom, &mut rng).unwrap();

        for i in 0..list_robot.len() {
            let merkle_proof = get_merkle_proof_from_id(
                Fr::from(i as u32),
                &merkle_proofs,
            );
            let _ = post(
                new_robot_root,
                task_root,
                &zkproof,
                list_new_robot[i],
                merkle_proof,
                &configuration.list_ip[i],
                "3_3"
            )
                .await;
        }

        list_robot = list_new_robot;
        root = new_robot_root;

        counter = counter + 1;

        if counter < configuration.iterations {
            thread::sleep(Duration::from_secs(configuration.clock));
        }
    }
    Ok(())
}

async fn post(
    robot_root: Fr,
    task_root: Fr,
    proof: &Proof<Bn254>,
    robot: Robot,
    merkle: Vec<Fr>,
    ip: &str,
    circuit: &str
) -> Result<(), Error> {
    let mut url = "http://".to_owned();
    url.push_str(&ip);
    url.push_str("/update");


    let mut json_data: String = r#"{"robot_root" : ""#.to_owned();
    json_data.push_str(&robot_root.into_bigint().to_string());
    json_data.push_str(r#"","task_root" : ""#);
    json_data.push_str(&task_root.into_bigint().to_string());
    json_data.push_str(r#"","circuit" : ""#);
    json_data.push_str(circuit);
    json_data.push_str(r#"","proof" : [""#);
    json_data.push_str(&proof.a.x.into_bigint().to_string());
    json_data.push_str(r#"",""#);
    json_data.push_str(&proof.a.y.into_bigint().to_string());
    json_data.push_str(r#"",""#);
    let mut b1 = proof.b.x.to_base_prime_field_elements();
    let mut b2 = proof.b.y.to_base_prime_field_elements();
    json_data.push_str(&b1.next().unwrap().into_bigint().to_string());
    json_data.push_str(r#"",""#);
    json_data.push_str(&b1.next().unwrap().into_bigint().to_string());
    json_data.push_str(r#"",""#);
    json_data.push_str(&b2.next().unwrap().into_bigint().to_string());
    json_data.push_str(r#"",""#);
    json_data.push_str(&b2.next().unwrap().into_bigint().to_string());
    json_data.push_str(r#"",""#);
    json_data.push_str(&proof.c.x.into_bigint().to_string());
    json_data.push_str(r#"",""#);
    json_data.push_str(&proof.c.y.into_bigint().to_string());
    json_data.push_str(r#""],"task_id" : ""#);
    json_data.push_str(&robot.list_tasks[1].task_id.into_bigint().to_string());
    json_data.push_str(r#"","x" : ""#);
    json_data.push_str(&robot.list_tasks[1].x.into_bigint().to_string());
    json_data.push_str(r#"","y" : ""#);
    json_data.push_str(&robot.list_tasks[1].y.into_bigint().to_string());
    json_data.push_str(r#"","z" : ""#);
    json_data.push_str(&robot.list_tasks[1].z.into_bigint().to_string());
    json_data.push_str(r#"","merkle_proof" : [""#);
    for i in 0..merkle.len() {
        json_data.push_str(&merkle[i].into_bigint().to_string());
        if i != merkle.len() - 1 {
            json_data.push_str(r#"",""#);
        }
    }
    json_data.push_str(r#""]}"#);

    let client = reqwest::Client::new();

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(json_data.to_owned())
        .send()
        .await?;

    let response_body = response.text().await?;
    println!(
        "Response of robot {}:\t{}",
        robot.robot_id.into_bigint().to_string(),
        response_body
    );

    Ok(())
}

async fn post_3_3(
    robot_root: Fr,
    task_root: Fr,
    proof: &Proof<Bn254>,
    robot: Robot,
    merkle: Vec<Fr>,
    ip: &str,
) -> Result<(), Error> {
    let mut url = "http://".to_owned();
    url.push_str(&ip);
    url.push_str("/update_3_3");

    let mut json_data: String = r#"{"robot_root" : ""#.to_owned();
    json_data.push_str(&robot_root.into_bigint().to_string());
    json_data.push_str(r#"","task_root" : ""#);
    json_data.push_str(&task_root.into_bigint().to_string());
    json_data.push_str(r#"","proof" : [""#);
    json_data.push_str(&proof.a.x.into_bigint().to_string());
    json_data.push_str(r#"",""#);
    json_data.push_str(&proof.a.y.into_bigint().to_string());
    json_data.push_str(r#"",""#);
    let mut b1 = proof.b.x.to_base_prime_field_elements();
    let mut b2 = proof.b.y.to_base_prime_field_elements();
    json_data.push_str(&b1.next().unwrap().into_bigint().to_string());
    json_data.push_str(r#"",""#);
    json_data.push_str(&b1.next().unwrap().into_bigint().to_string());
    json_data.push_str(r#"",""#);
    json_data.push_str(&b2.next().unwrap().into_bigint().to_string());
    json_data.push_str(r#"",""#);
    json_data.push_str(&b2.next().unwrap().into_bigint().to_string());
    json_data.push_str(r#"",""#);
    json_data.push_str(&proof.c.x.into_bigint().to_string());
    json_data.push_str(r#"",""#);
    json_data.push_str(&proof.c.y.into_bigint().to_string());
    json_data.push_str(r#""],"task_id" : ""#);
    json_data.push_str(&robot.list_tasks[1].task_id.into_bigint().to_string());
    json_data.push_str(r#"","x" : ""#);
    json_data.push_str(&robot.list_tasks[1].x.into_bigint().to_string());
    json_data.push_str(r#"","y" : ""#);
    json_data.push_str(&robot.list_tasks[1].y.into_bigint().to_string());
    json_data.push_str(r#"","z" : ""#);
    json_data.push_str(&robot.list_tasks[1].z.into_bigint().to_string());
    json_data.push_str(r#"","merkle_proof" : [""#);
    for i in 0..merkle.len() {
        json_data.push_str(&merkle[i].into_bigint().to_string());
        if i != merkle.len() - 1 {
            json_data.push_str(r#"",""#);
        }
    }
    json_data.push_str(r#""]}"#);

    let client = reqwest::Client::new();

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(json_data.to_owned())
        .send()
        .await?;

    let response_body = response.text().await?;
    println!(
        "Response of robot {}:\n{}",
        robot.robot_id.into_bigint().to_string(),
        response_body
    );

    Ok(())
}