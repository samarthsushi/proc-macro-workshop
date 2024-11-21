// Write code here.
//
// To see what the code looks like after macro expansion:
//     $ cargo expand
//
// To run the code:
//     $ cargo run
use derive_builder::Builder;

type Option = ();
type Some = ();
type None = ();
type Result = ();
type Box = ();

#[derive(Builder)]
pub struct Command {
    executable: String,
<<<<<<< HEAD
=======
    #[builder(eac = "arg")]
    args: Vec<String>,
    env: Vec<String>,
    current_dir: Option<String>,
>>>>>>> a09bb46030ca8f4dd8b6d47657fc876193ff5ce6
}

fn main() {}

