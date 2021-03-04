use std::{
    ffi::{CStr, CString},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::*;

struct ServerData(AtomicBool);

impl<S: ServerBackend> ServerObjectData<S> for ServerData {
    fn make_child(self: Arc<Self>, _: &ObjectInfo) -> Arc<dyn ServerObjectData<S>> {
        self
    }

    fn request(
        &self,
        _: &mut S::Handle,
        _: S::ClientId,
        _: S::ObjectId,
        opcode: u16,
        arguments: &[Argument<S::ObjectId>],
    ) {
        assert_eq!(opcode, 0);
        if let [Argument::Uint(u), Argument::Int(i), Argument::Fixed(f), Argument::Array(ref a), Argument::Str(ref s), Argument::Fd(fd)] =
            arguments
        {
            assert_eq!(*u, 42);
            assert_eq!(*i, -13);
            assert_eq!(*f, 4589);
            assert_eq!(&**a, &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
            assert_eq!(&***s, CStr::from_bytes_with_nul(b"I like trains\0").unwrap());
            // compare the fd to stdin
            let stat1 = ::nix::sys::stat::fstat(*fd).unwrap();
            let stat2 = ::nix::sys::stat::fstat(0).unwrap();
            assert_eq!(stat1.st_dev, stat2.st_dev);
            assert_eq!(stat1.st_ino, stat2.st_ino);
        } else {
            panic!("Bad argument list !")
        }
        self.0.store(true, Ordering::SeqCst);
    }

    fn destroyed(&self, _: S::ClientId, _: S::ObjectId) {}
}

impl<S: ServerBackend> GlobalHandler<S> for ServerData {
    fn make_data(self: Arc<Self>, _: &ObjectInfo) -> Arc<dyn ServerObjectData<S>> {
        self
    }

    fn bind(&self, _: S::ClientId, _: S::GlobalId, _: S::ObjectId) {}
}

// create a global and send the many_args method
fn test<C: ClientBackend, S: ServerBackend + ServerPolling<S>>() {
    let mut test = TestPair::<C, S>::init();

    let server_data = Arc::new(ServerData(AtomicBool::new(false)));

    // Prepare a global
    test.server.handle().create_global(&interfaces::TEST_GLOBAL_INTERFACE, 1, server_data.clone());

    // get the registry client-side
    let client_display = test.client.handle().display_id();
    let placeholder =
        test.client.handle().placeholder_id(Some((&interfaces::WL_REGISTRY_INTERFACE, 1)));
    let registry_id = test
        .client
        .handle()
        .send_constructor(
            client_display,
            1,
            &[Argument::NewId(placeholder)],
            Some(Arc::new(DoNothingData)),
        )
        .unwrap();
    // create the test global
    let placeholder =
        test.client.handle().placeholder_id(Some((&interfaces::TEST_GLOBAL_INTERFACE, 1)));
    let test_global_id = test
        .client
        .handle()
        .send_constructor(
            registry_id,
            0,
            &[
                Argument::Uint(1),
                Argument::Str(Box::new(
                    CString::new(interfaces::TEST_GLOBAL_INTERFACE.name.as_bytes()).unwrap(),
                )),
                Argument::Uint(1),
                Argument::NewId(placeholder),
            ],
            None,
        )
        .unwrap();
    // send the many_args request
    test.client
        .handle()
        .send_request(
            test_global_id,
            0,
            &[
                Argument::Uint(42),
                Argument::Int(-13),
                Argument::Fixed(4589),
                Argument::Array(Box::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9])),
                Argument::Str(Box::new(CString::new("I like trains".as_bytes()).unwrap())),
                Argument::Fd(0), // stdin
            ],
        )
        .unwrap();
    test.client_flush().unwrap();

    test.server_dispatch().unwrap();

    assert!(server_data.0.load(Ordering::SeqCst));
}

expand_test!(test);
