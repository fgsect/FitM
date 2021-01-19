use fitm::get_traces;

#[test]
fn test_get_traces() {
    let traces = get_traces();
    println!("{:?}", traces);
}
