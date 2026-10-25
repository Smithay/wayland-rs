use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(unix)]
use std::{io, os::unix::io::AsFd};

use super::*;

struct ServerData(AtomicBool);

macro_rules! serverdata_impls {
    ($server_backend:tt) => {
        impl $server_backend::ObjectData<()> for ServerData {
            fn request(
                self: Arc<Self>,
                _: &$server_backend::Handle,
                _: &mut (),
                _: &$server_backend::ClientId,
                msg: OwnedMessage<$server_backend::ObjectId>,
            ) -> Option<Arc<dyn $server_backend::ObjectData<()>>> {
                #[cfg_attr(windows, allow(unused_variables))]
                let (u, i, f, a, s, fd) = match (msg.opcode, &msg.args[..]) {
                    #[cfg(unix)]
                    (
                        0,
                        [
                            OwnedArgument::Uint(u),
                            OwnedArgument::Int(i),
                            OwnedArgument::Fixed(f),
                            OwnedArgument::Array(a),
                            OwnedArgument::Str(Some(s)),
                            OwnedArgument::Fd(fd),
                        ],
                    ) => (u, i, f, a, s, Some(fd)),
                    (
                        7,
                        [
                            OwnedArgument::Uint(u),
                            OwnedArgument::Int(i),
                            OwnedArgument::Fixed(f),
                            OwnedArgument::Array(a),
                            OwnedArgument::Str(Some(s)),
                        ],
                    ) => (
                        u,
                        i,
                        f,
                        a,
                        s,
                        #[cfg(unix)]
                        None,
                        #[cfg(windows)]
                        Option::<()>::None,
                    ),
                    _ => panic!("Bad argument list !"),
                };
                assert_eq!(*u, 42);
                assert_eq!(*i, -13);
                assert_eq!(*f, 4589);
                assert_eq!(&**a, &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
                assert_eq!(&***s, "I like trains");
                #[cfg(unix)]
                {
                    // compare the fd to stdin
                    let stat1 = rustix::fs::fstat(&fd).unwrap();
                    let stat2 = rustix::fs::fstat(std::io::stdin()).unwrap();
                    assert_eq!(stat1.st_dev, stat2.st_dev);
                    assert_eq!(stat1.st_ino, stat2.st_ino);
                }
                self.0.store(true, Ordering::SeqCst);
                None
            }

            fn destroyed(
                self: Arc<Self>,
                _: &$server_backend::Handle,
                _: &mut (),
                _: &$server_backend::ClientId,
                _: &$server_backend::ObjectId,
            ) {
            }
        }

        impl $server_backend::GlobalHandler<()> for ServerData {
            fn bind(
                self: Arc<Self>,
                handle: &$server_backend::Handle,
                _: &mut (),
                _: &$server_backend::ClientId,
                _: &$server_backend::GlobalId,
                object_id: &$server_backend::ObjectId,
            ) -> Arc<dyn $server_backend::ObjectData<()>> {
                #[cfg(unix)]
                let stdout = io::stdout().lock();
                handle
                    .send_event(
                        #[cfg(unix)]
                        message!(
                            &object_id,
                            0,
                            [
                                Argument::Uint(1337),
                                Argument::Int(-53),
                                Argument::Fixed(9823),
                                Argument::Array(Box::new(&[10, 20, 30, 40, 50, 60, 70, 80, 90])),
                                Argument::Str(Some(Box::new("I want cake"))),
                                Argument::Fd(stdout.as_fd()),
                            ],
                        ),
                        #[cfg(windows)]
                        message!(
                            &object_id,
                            3,
                            [
                                Argument::Uint(1337),
                                Argument::Int(-53),
                                Argument::Fixed(9823),
                                Argument::Array(Box::new(&[10, 20, 30, 40, 50, 60, 70, 80, 90])),
                                Argument::Str(Some(Box::new("I want cake"))),
                            ],
                        ),
                    )
                    .unwrap();
                self
            }
        }
    };
}

serverdata_impls!(server_rs);
#[cfg(unix)]
serverdata_impls!(server_sys);

struct ClientData(AtomicBool);

macro_rules! clientdata_impls {
    ($client_backend:tt) => {
        impl $client_backend::ObjectData for ClientData {
            fn event(
                self: Arc<Self>,
                _handle: &$client_backend::Backend,
                msg: OwnedMessage<$client_backend::ObjectId>,
            ) -> Option<Arc<dyn $client_backend::ObjectData>> {
                #[cfg_attr(windows, allow(unused_variables))]
                let (u, i, f, a, s, fd) = match (msg.opcode, &msg.args[..]) {
                    #[cfg(unix)]
                    (
                        0,
                        [
                            OwnedArgument::Uint(u),
                            OwnedArgument::Int(i),
                            OwnedArgument::Fixed(f),
                            OwnedArgument::Array(a),
                            OwnedArgument::Str(Some(s)),
                            OwnedArgument::Fd(fd),
                        ],
                    ) => (u, i, f, a, s, Some(fd)),
                    (
                        3,
                        [
                            OwnedArgument::Uint(u),
                            OwnedArgument::Int(i),
                            OwnedArgument::Fixed(f),
                            OwnedArgument::Array(a),
                            OwnedArgument::Str(Some(s)),
                        ],
                    ) => (
                        u,
                        i,
                        f,
                        a,
                        s,
                        #[cfg(unix)]
                        None,
                        #[cfg(windows)]
                        Option::<()>::None,
                    ),
                    _ => panic!("Bad argument list !"),
                };
                assert_eq!(*u, 1337);
                assert_eq!(*i, -53);
                assert_eq!(*f, 9823);
                assert_eq!(&**a, &[10, 20, 30, 40, 50, 60, 70, 80, 90]);
                assert_eq!(&***s, "I want cake");
                #[cfg(unix)]
                if let Some(fd) = fd {
                    // compare the fd to stdout
                    let stat1 = rustix::fs::fstat(&fd).unwrap();
                    let stat2 = rustix::fs::fstat(std::io::stdout()).unwrap();
                    assert_eq!(stat1.st_dev, stat2.st_dev);
                    assert_eq!(stat1.st_ino, stat2.st_ino);
                }
                self.0.store(true, Ordering::SeqCst);
                None
            }
            fn destroyed(&self, _object_id: &$client_backend::ObjectId) {}
        }
    };
}

clientdata_impls!(client_rs);
#[cfg(unix)]
clientdata_impls!(client_sys);

// create a global and send the many_args method
expand_test!(many_args, {
    let (tx, rx) = UnixStream::pair_named(file!(), line!()).unwrap();
    let server = server_backend::Backend::new().unwrap();
    let _client_id = server.handle().insert_client(rx, Arc::new(())).unwrap();
    let client = client_backend::Backend::connect(tx).unwrap();

    let server_data = Arc::new(ServerData(AtomicBool::new(false)));
    let client_data = Arc::new(ClientData(AtomicBool::new(false)));

    // Prepare a global
    server.handle().create_global(&interfaces::TEST_GLOBAL_INTERFACE, 1, server_data.clone());

    // get the registry client-side
    let client_display = client.display_id();
    let registry_id = client
        .send_request(
            message!(&client_display, 1, [Argument::NewId(client_backend::ObjectId::null())],),
            Some(Arc::new(DoNothingData)),
            Some((&interfaces::WL_REGISTRY_INTERFACE, 1)),
        )
        .unwrap();
    // create the test global
    let test_global_id = client
        .send_request(
            message!(
                &registry_id,
                0,
                [
                    Argument::Uint(1),
                    Argument::Str(Some(Box::new(interfaces::TEST_GLOBAL_INTERFACE.name))),
                    Argument::Uint(1),
                    Argument::NewId(client_backend::ObjectId::null()),
                ],
            ),
            Some(client_data.clone()),
            Some((&interfaces::TEST_GLOBAL_INTERFACE, 1)),
        )
        .unwrap();

    client.flush().unwrap();
    server.dispatch_all_clients(&mut ()).unwrap();
    server.flush(None).unwrap();
    client.prepare_read().unwrap().read().unwrap();
    assert!(client_data.0.load(Ordering::SeqCst));

    // send the many_args request
    #[cfg(unix)]
    let stdin = io::stdin().lock();
    #[cfg(unix)]
    let message = message!(
        &test_global_id,
        0,
        [
            Argument::Uint(42),
            Argument::Int(-13),
            Argument::Fixed(4589),
            Argument::Array(Box::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9])),
            Argument::Str(Some(Box::new("I like trains"))),
            Argument::Fd(stdin.as_fd()),
        ],
    );
    #[cfg(windows)]
    let message = message!(
        &test_global_id,
        7,
        [
            Argument::Uint(42),
            Argument::Int(-13),
            Argument::Fixed(4589),
            Argument::Array(Box::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9])),
            Argument::Str(Some(Box::new("I like trains"))),
        ],
    );
    client.send_request(message, None, None).unwrap();
    client.flush().unwrap();

    server.dispatch_all_clients(&mut ()).unwrap();

    assert!(server_data.0.load(Ordering::SeqCst));
});
