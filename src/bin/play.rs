use p4_cmd::P4;

fn main() {
    let p4 = P4::new();
    println!("{:?}", p4.print("//depot/...#have"));
}