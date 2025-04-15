use serde::Serialize;
use std::error::Error;
use std::fs::File;

#[derive(Serialize)]
struct Employee {
    id: u32,
    name: String,
    department: String,
    salary: f64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let employees = vec![
        Employee {
            id: 1,
            name: "Alice".to_string(),
            department: "Engineering".to_string(),
            salary: 75000.0,
        },
        Employee {
            id: 2,
            name: "Bob".to_string(),
            department: "Marketing".to_string(),
            salary: 65000.0,
        },
        Employee {
            id: 3,
            name: "Charlie".to_string(),
            department: "HR".to_string(),
            salary: 60000.0,
        },
    ];

    let file = File::create("employees.csv")?;
    let mut wtr = csv::Writer::from_writer(file);

    for employee in employees {
        wtr.serialize(employee)?;
    }

    wtr.flush()?;
    println!("CSV 文件已成功写入！");
    Ok(())
}
