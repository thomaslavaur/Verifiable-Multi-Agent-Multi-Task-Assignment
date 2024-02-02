use ark_bn254::Fr;
use ark_ff::PrimeField;
use std::fmt::{Display, Formatter};
use ark_std::iterable::Iterable;

#[derive(Debug, Clone, Copy)]
pub struct Task {
    pub task_id: Fr,
    pub x: Fr, // task position (meters) between 0 and 10 000
    pub y: Fr, // task position (meters) between 0 and 10 000
    pub z: Fr, // task position (meters) between 0 and 10 000
}

#[derive(Debug, Clone, Copy)]
pub struct Robot {
    pub robot_id: Fr,
    pub list_tasks: [Task; 2],
}

impl Display for Task {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.task_id.into_bigint().to_string())
    }
}

impl Display for Robot {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.robot_id.into_bigint().to_string())
    }
}

pub fn create_task(task_id: Fr, positions: [Fr; 3]) -> Task {
    Task {
        task_id,
        x: positions[0],
        y: positions[1],
        z: positions[2],
    }
}

pub fn create_robot(robot_id: Fr, task_id: [Fr; 2], positions: [[Fr; 3]; 2]) -> Robot {
    Robot {
        robot_id,
        list_tasks: [
            create_task(task_id[0], positions[0]),
            create_task(task_id[1], positions[1]),
        ],
    }
}

pub fn create_list_tasks(
    task_id: &Vec<Fr>,
    positions: &Vec<[Fr; 3]>,
    log_length: usize,
) -> Vec<Task> {
    assert_eq!(task_id.len(), 2usize.pow(log_length as u32));
    assert_eq!(positions.len(), 2usize.pow(log_length as u32));

    let mut tasks = vec![
        create_task(Fr::from(0), [Fr::from(0), Fr::from(0), Fr::from(0)]);
        2usize.pow(log_length as u32)
    ];
    for i in 0..2usize.pow(log_length as u32) {
        tasks[i] = create_task(task_id[i], positions[i]);
    }

    tasks
}

pub fn create_list_robots(
    robots_id: &Vec<Fr>,
    tasks_id: &Vec<[Fr; 2]>,
    positions: &Vec<[[Fr; 3]; 2]>,
    log_length: usize,
) -> Vec<Robot> {
    assert_eq!(robots_id.len(), 2usize.pow(log_length as u32));
    assert_eq!(tasks_id.len(), 2usize.pow(log_length as u32));
    assert_eq!(positions.len(), 2usize.pow(log_length as u32));

    let mut robots = vec![
        create_robot(Fr::from(0), [Fr::from(0); 2], [[Fr::from(0); 3]; 2]);
        2usize.pow(log_length as u32)
    ];
    for i in 0..2usize.pow(log_length as u32) {
        robots[i] = create_robot(robots_id[i], tasks_id[i], positions[i]);
    }

    robots
}

fn distance(task: &Task, robot: &Robot) -> Fr {
    (robot.list_tasks[1].x - task.x) * (robot.list_tasks[1].x - task.x)
        + (robot.list_tasks[1].y - task.y) * (robot.list_tasks[1].y - task.y)
        + (robot.list_tasks[1].z - task.z) * (robot.list_tasks[1].z - task.z)
}

fn cost_matrix(robots: &Vec<Robot>, tasks: &Vec<Task>) -> Vec<Vec<Fr>> {
    let mut cost =
        vec![vec![Fr::from(0); tasks.len()]; robots.len()];
    for i in 0..robots.len() {
        for j in 0..tasks.len() {
            cost[i][j] = distance(&tasks[j], &robots[i]);
        }
    }

    cost
}

fn min(array: Vec<Fr>) -> Fr {
    let mut min = array[0];
    for i in 0..array.len() {
        if array[i] < min {
            min = array[i];
        }
    }
    min
}

fn index(array: &Vec<Fr>, min: Fr) -> Fr {
    let mut idx = Fr::from(array.len() as i32);
    for i in 0..array.len() {
        if array[i] == min {
            idx = Fr::from(i as i32);
        }
    }
    idx
}

fn rank_matrix(cost: &Vec<Vec<Fr>>) -> Vec<Vec<Fr>> {
    let mut rank =
        vec![vec![Fr::from(0); cost[0].len()]; cost.len()];
    let mut dist = cost.clone();
    let mut tmp: Fr;

    for i in 0..cost.len() {
        for j in 0..cost[0].len() {
            rank[i][j] = Fr::from(i as i32);
        }
    }

    for j in 0..cost[0].len() {
        for i in 0..cost.len() {
            for k in (1 + i)..cost.len() {
                if dist[k][j] < dist[i][j] {
                    tmp = dist[i][j];
                    dist[i][j] = dist[k][j];
                    dist[k][j] = tmp;
                    tmp = rank[i][j];
                    rank[i][j] = rank[k][j];
                    rank[k][j] = tmp;
                }
            }
        }
    }

    rank
}

fn task_choice(
    rank: &Vec<Fr>,
    available: &Vec<bool>,
    robot_id: Fr,
    cost: &Vec<Fr>,
) -> Fr {
    let mut array = vec![Fr::from(0); rank.len()];
    for i in 0..rank.len() {
        if rank[i] == robot_id && available[i] {
            array[i] = cost[i];
        } else {
            array[i] = Fr::from(399999999i32);
        }
    }

    let min = min(array);
    index(cost, min)
}

fn assign(rank: &Vec<Vec<Fr>>, cost: &Vec<Vec<Fr>>) -> Vec<Fr> {
    let mut robot_available = vec![true; cost.len()];
    let mut task_available = vec![true; cost[0].len()];
    let mut task: Fr;
    let mut assignments = vec![Fr::from(0); cost.len()];

    for i in 0..cost.len() {
        for j in 0..cost.len() {
            task = task_choice(
                &rank[i],
                &task_available,
                Fr::from(j as i32),
                &cost[i]
            );
            if task != Fr::from(cost[0].len() as u32) && robot_available[j] {
                robot_available[j] = false;
                task_available[task.into_bigint().to_string().parse::<usize>().unwrap()] = false;
                assignments[j] = task;
            }
        }
    }

    assignments
}

pub fn skate(robots: &Vec<Robot>, tasks: &Vec<Task>) -> Vec<Robot> {

    let cost = cost_matrix(&robots, &tasks);
    let rank = rank_matrix(&cost);
    let assignments = assign(&rank, &cost);

    let mut new_robots = robots.clone();
    for i in 0..robots.len() {
        new_robots[i].list_tasks[0] = new_robots[i].list_tasks[1];
        new_robots[i].list_tasks[1].task_id = assignments[i];
        new_robots[i].list_tasks[1].x = tasks[assignments[i]
            .into_bigint()
            .to_string()
            .parse::<usize>()
            .unwrap()]
        .x;
        new_robots[i].list_tasks[1].y = tasks[assignments[i]
            .into_bigint()
            .to_string()
            .parse::<usize>()
            .unwrap()]
        .y;
        new_robots[i].list_tasks[1].z = tasks[assignments[i]
            .into_bigint()
            .to_string()
            .parse::<usize>()
            .unwrap()]
        .z;
    }
    new_robots
}
