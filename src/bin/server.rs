// ROBOTS (or taxi or any agent)

use ark_bn254::{Bn254, Fq, Fq2, Fr, G1Affine, G2Affine};
use ark_circom::CircomReduction;
use ark_crypto_primitives::snark::SNARK;
use ark_ff::{Field, PrimeField};
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof, VerifyingKey};
use ark_std::Zero;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::from_str;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
//use rosrust::Message;
use tokio;
use tokio::sync::{Mutex, MutexGuard};
use SKATE::hashes::verify_robot_in_tree;
use SKATE::Skate::{Robot, Task};

type GrothBn = Groth16<Bn254, CircomReduction>;

#[derive(Debug, Deserialize)]
struct ServerConfig {
    ip: String,
    number_of_agent: usize,
    robot_id: String,
    robot_root: String,
    x: String,
    y: String,
    z: String,
    scale: f64, //max in cm
}

#[tokio::main]
async fn main() {
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
    let configuration: ServerConfig = match toml::from_str(&toml_content) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Cannot deserialize the configuration file : {}", e);
            std::process::exit(1);
        }
    };

    assert!(configuration.x.parse::<Fr>().unwrap() < Fr::from(10000));
    assert!(configuration.y.parse::<Fr>().unwrap() < Fr::from(10000));
    assert!(configuration.z.parse::<Fr>().unwrap() < Fr::from(10000));

    tracing_subscriber::fmt::init();

    let verifier_key_6_3 = read_verifying_key("verification_key_6_3.json".to_owned());
    let verifier_key_3_3 = read_verifying_key("verification_key_3_3.json".to_owned());

    rosrust::init("talker");
    let chatter_pub = rosrust::publish("replace_by_corect_topic", 100).unwrap();

    let state = AppState {
        robot: Mutex::new(Robot {
            robot_id: configuration.robot_id.parse::<Fr>().unwrap(),
            list_tasks: [
                Task {
                    task_id: Fr::from(0),
                    x: Fr::from(0),
                    y: Fr::from(0),
                    z: Fr::from(0),
                },
                Task {
                    task_id: Fr::from(1),
                    x: configuration.x.parse::<Fr>().unwrap(),
                    y: configuration.y.parse::<Fr>().unwrap(),
                    z: configuration.z.parse::<Fr>().unwrap(),
                },
            ],
        }),
        root: Mutex::new(configuration.robot_root.parse::<Fr>().unwrap()),
        key_6_3: Mutex::new(GrothBn::process_vk(&verifier_key_6_3).unwrap()),
        key_3_3: Mutex::new(GrothBn::process_vk(&verifier_key_3_3).unwrap()),
        number_of_robots: configuration.number_of_agent,
        scale: configuration.scale,
        publisher: chatter_pub
    };

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        // `POST /users` goes to `create_user`
        .route("/update", post(update))
        .with_state(Arc::new(state));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(configuration.ip)
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
async fn update(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Update>,
) -> (StatusCode, Json<String>) {
    // insert your application logic here

    let mut root = state.root.lock().await;
    let mut robot = state.robot.lock().await;
    let mut key: MutexGuard<PreparedVerifyingKey<Bn254>>;

    if payload.circuit == "6_3".to_owned() {
        key = state.key_6_3.lock().await;
    } else {
        key = state.key_3_3.lock().await;
    }

    let mut inputs = [Fr::from(0); 3];
    inputs[0] = payload.robot_root.parse::<Fr>().unwrap();
    inputs[1] = *root;
    inputs[2] = payload.task_root.parse::<Fr>().unwrap();

    let mut proof = Proof::default();
    proof.a = to_g1(vec![payload.proof[0].clone(), payload.proof[1].clone()]);
    proof.b = to_g2(vec![
        vec![payload.proof[2].clone(), payload.proof[3].clone()],
        vec![payload.proof[4].clone(), payload.proof[5].clone()],
    ]);
    proof.c = to_g1(vec![payload.proof[6].clone(), payload.proof[7].clone()]);

    let correct_proof = GrothBn::verify_with_processed_vk(&key, &inputs, &proof).unwrap();

    let mut merkle = vec![];
    for i in 0..state.number_of_robots.ilog2() as usize + 1 {
        merkle.push(payload.merkle_proof[i].parse::<Fr>().unwrap());
    }
    let mut new_robot = robot.clone();
    new_robot.list_tasks[0] = robot.list_tasks[1].clone();
    new_robot.list_tasks[1].task_id = payload.task_id.parse::<Fr>().unwrap();
    new_robot.list_tasks[1].x = payload.x.parse::<Fr>().unwrap();
    new_robot.list_tasks[1].y = payload.y.parse::<Fr>().unwrap();
    new_robot.list_tasks[1].z = payload.z.parse::<Fr>().unwrap();
    let in_tree = verify_robot_in_tree(
        new_robot,
        payload.robot_root.parse::<Fr>().unwrap(),
        merkle,
    );

    if in_tree && correct_proof {
        robot.list_tasks[0] = robot.list_tasks[1].clone();
        robot.list_tasks[1].task_id = payload.task_id.parse::<Fr>().unwrap();
        robot.list_tasks[1].x = payload.x.parse::<Fr>().unwrap();
        robot.list_tasks[1].y = payload.y.parse::<Fr>().unwrap();
        robot.list_tasks[1].z = payload.z.parse::<Fr>().unwrap();
        *root = payload.robot_root.parse::<Fr>().unwrap();

        println!(
            "Preuve ok. Assigned Task:\tid: {}\tx: {}\ty: {}\tz: {}",
            robot.list_tasks[1].task_id.into_bigint().to_string(),
            (robot.list_tasks[1].x.into_bigint().to_string().parse::<f64>().unwrap() / 10000f64) * state.scale - 2f64,
            (robot.list_tasks[1].y.into_bigint().to_string().parse::<f64>().unwrap() / 10000f64) * state.scale - 2f64,
            (robot.list_tasks[1].z.into_bigint().to_string().parse::<f64>().unwrap() / 10000f64) * state.scale - 2f64
        );

        //ROS publish

        let mut msg = rosrust_msg::geometry_msgs::Point::default();
        msg.x = (robot.list_tasks[1].x.into_bigint().to_string().parse::<f64>().unwrap() / 10000f64) * 4f64 - 2f64;
        msg.y = (robot.list_tasks[1].y.into_bigint().to_string().parse::<f64>().unwrap() / 10000f64) * 4f64 - 2f64;
        msg.z = (robot.list_tasks[1].z.into_bigint().to_string().parse::<f64>().unwrap() / 10000f64) * 4f64 - 2f64;

        // Send string message to topic via publisher
        state.publisher.send(msg).unwrap();




        (StatusCode::OK, Json("OK".to_string()))
    } else {
        if in_tree {
            println!("Incorrect ZkProof");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Zk Proof is incorrect".to_string()),
            )
        } else {
            if correct_proof {
                println!("Incorrect Merkle Proof");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json("Merkle Proof is incorrect".to_string()),
                )
            } else {
                println!("Incorrect ZkProof and Merkle Proof");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json("Zk Proof and Merkle Proof are incorrect".to_string()),
                )
            }
        }
    }
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct Update {
    robot_root: String,
    task_root: String,
    circuit: String,
    proof: Vec<String>,
    task_id: String,
    x: String,
    y: String,
    z: String,
    merkle_proof: Vec<String>,
}

fn read_verifying_key(directory: String) -> VerifyingKey<Bn254> {
    let mut file = File::open(
        std::env::current_dir()
            .unwrap()
            .join(PathBuf::from(directory)),
    )
    .expect("Impossible d'ouvrir le fichier");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Impossible de lire le fichier");

    let mut key = VerifyingKey::default();
    let json: VerifierFile = from_str(&contents).expect("Impossible de désérialiser le JSON");
    key.alpha_g1 = to_g1(json.vk_alpha_1);
    key.beta_g2 = to_g2(json.vk_beta_2);
    key.delta_g2 = to_g2(json.vk_delta_2);
    key.gamma_g2 = to_g2(json.vk_gamma_2);
    key.gamma_abc_g1 = to_g1_vec(json.IC, 4);
    key
}

#[derive(Debug, Deserialize)]
struct VerifierFile {
    protocol: String,
    curve: String,
    nPublic: u32,
    vk_alpha_1: Vec<String>,
    vk_beta_2: Vec<Vec<String>>,
    vk_gamma_2: Vec<Vec<String>>,
    vk_delta_2: Vec<Vec<String>>,
    vk_alphabeta_12: Vec<Vec<Vec<String>>>,
    IC: Vec<Vec<String>>,
}

fn to_g1(string: Vec<String>) -> G1Affine {
    let x = string[0].parse::<Fq>().unwrap();
    let y = string[1].parse::<Fq>().unwrap();
    let infinity = x.is_zero() && y.is_zero();
    if infinity {
        G1Affine::identity()
    } else {
        G1Affine::new_unchecked(x, y)
    }
}

fn to_g2(string: Vec<Vec<String>>) -> G2Affine {
    let f1 = Fq2::from_base_prime_field_elems(&[
        string[0][0].parse::<Fq>().unwrap(),
        string[0][1].parse::<Fq>().unwrap(),
    ])
    .unwrap();
    let f2 = Fq2::from_base_prime_field_elems(&[
        string[1][0].parse::<Fq>().unwrap(),
        string[1][1].parse::<Fq>().unwrap(),
    ])
    .unwrap();
    let infinity = f1.is_zero() && f2.is_zero();
    if infinity {
        G2Affine::identity()
    } else {
        G2Affine::new_unchecked(f1, f2)
    }
}

fn to_g1_vec(string: Vec<Vec<String>>, n_vars: usize) -> Vec<G1Affine> {
    (0..n_vars).map(|i| to_g1(string[i].clone())).collect()
}

struct AppState {
    robot: Mutex<Robot>,
    root: Mutex<Fr>,
    key_6_3: Mutex<PreparedVerifyingKey<Bn254>>,
    key_3_3: Mutex<PreparedVerifyingKey<Bn254>>,
    number_of_robots: usize,
    scale: f64,
    publisher: rosrust::Publisher<rosrust_msg::geometry_msgs::Point>
}
