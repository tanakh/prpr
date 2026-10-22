struct Tree;

prpr::grammar! {
    #[memo]
    tree: Tree = "x" { Tree }
    //~^ ERROR: the trait bound `Tree: Clone` is not satisfied
}

fn main() {}
