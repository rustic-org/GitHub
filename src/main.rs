#[actix_rt::main]
async fn main() {
    match github::start().await {
        Ok(_) => {
            println!("\nGitHub session has ended")
        }
        Err(err) => {
            eprintln!("\nError starting GitHub: {}", err)
        }
    }
}
