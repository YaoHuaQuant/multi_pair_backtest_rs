fn main() {
    let arr = [1, 2, 3, 4, 5];
    let direction = true; // true 表示正向遍历，false 表示反向遍历

    let iter: Box<dyn Iterator<Item = &i32>> = if direction {
        Box::new(arr.iter())
    } else {
        Box::new(arr.iter().rev())
    };

    for item in iter {
        println!("{}", item);
    }
}
