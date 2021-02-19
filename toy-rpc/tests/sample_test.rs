mod rpc;

#[cfg(feature = "std")]
#[test]
fn test_mod() {
    rpc::hello_from_rpc_rs();
}
