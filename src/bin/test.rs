use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone)]
struct Task {
    priority: i32,
    name: String,
}

fn main() {
    let mut heap = BinaryHeap::new();
    let mut index_map = HashMap::new();

    // 插入任务，同时存储索引
    let tasks = vec![
        Task { priority: 3, name: "Task A".to_string() },
        Task { priority: 5, name: "Task B".to_string() },
        Task { priority: 1, name: "Task C".to_string() },
    ];

    for task in tasks {
        index_map.insert(task.name.clone(), task.clone());
        heap.push(task);
    }

    println!("Before modification: {:?}", heap);

    // 修改 "Task A"
    let target_name = "Task A";
    if let Some(mut task) = index_map.remove(target_name) {
        task.priority = 10; // 修改优先级
        heap.push(task.clone());
        index_map.insert(task.name.clone(), task);
    }

    println!("After modification: {:?}", heap);
}
