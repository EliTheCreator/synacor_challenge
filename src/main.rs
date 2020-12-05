use std::fs;

fn main() {
    let mut file = fs::read("challenge.bin").unwrap();
    let mut ip = 0;

    loop {
        let op = file[ip];
        let a = file[ip + 2];
        let b = file[ip + 4];
        let c = file[ip + 6];

        match op {
            0 => break,
            19 => {
                print!("{}", a as char);
                ip += 4
            }
            21 => ip += 4,
            _ => break,
        }
    }
}
