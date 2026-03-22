//! ServerBuilder tests

use crate::ServerBuilder;
use crate::mocks::{MockCamera, MockFilterWheel, MockSdk};
use eyre::eyre;

#[tokio::test]
async fn server_builder_default() {
    let builder = ServerBuilder::default();
    assert_eq!(builder.port, 0);
}

#[tokio::test]
async fn server_builder_new() {
    let builder = ServerBuilder::new();
    assert_eq!(builder.port, 0);
}

#[tokio::test]
async fn server_builder_with_port() {
    let builder = ServerBuilder::new().with_port(8080);
    assert_eq!(builder.port, 8080);
}

/// All build() tests are in a single test function because MockSdk::new() uses
/// a global static mock context that races when tests run in parallel.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn server_builder_build() {
    // -- sdk_new_fails --
    {
        let ctx = MockSdk::new_context();
        ctx.expect()
            .once()
            .returning(|| Err(eyre!("SDK init failed")));
        let result = ServerBuilder::new().build().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SDK init failed"));
    }

    // -- version_fails --
    {
        let ctx = MockSdk::new_context();
        ctx.expect().once().returning(|| {
            let mut sdk = MockSdk::default();
            sdk.expect_version()
                .once()
                .returning(|| Err(eyre!("version error")));
            Ok(sdk)
        });
        let result = ServerBuilder::new().build().await;
        assert!(result.is_err());
    }

    // -- no_devices --
    {
        let ctx = MockSdk::new_context();
        ctx.expect().once().returning(|| {
            let mut sdk = MockSdk::default();
            sdk.expect_version().once().returning(|| {
                Ok(qhyccd_rs::SDKVersion {
                    year: 2024,
                    month: 1,
                    day: 1,
                    subday: 0,
                })
            });
            sdk.expect_cameras()
                .once()
                .returning(|| Box::new(Vec::<MockCamera>::new().into_iter()));
            sdk.expect_filter_wheels()
                .once()
                .returning(|| Box::new(Vec::<MockFilterWheel>::new().into_iter()));
            Ok(sdk)
        });
        let result = ServerBuilder::new().with_port(0).build().await;
        assert!(result.is_ok());
    }

    // -- with_camera --
    {
        let ctx = MockSdk::new_context();
        ctx.expect().once().returning(|| {
            let mut sdk = MockSdk::default();
            sdk.expect_version().once().returning(|| {
                Ok(qhyccd_rs::SDKVersion {
                    year: 2024,
                    month: 1,
                    day: 1,
                    subday: 0,
                })
            });
            sdk.expect_cameras().once().returning(|| {
                let mut cam = MockCamera::new();
                cam.expect_id().return_const("QHY600-abc123".to_owned());
                cam.expect_clone().returning(MockCamera::new);
                Box::new(vec![cam].into_iter())
            });
            sdk.expect_filter_wheels()
                .once()
                .returning(|| Box::new(Vec::<MockFilterWheel>::new().into_iter()));
            Ok(sdk)
        });
        let result = ServerBuilder::new().with_port(0).build().await;
        assert!(result.is_ok());
    }

    // -- with_filter_wheel --
    {
        let ctx = MockSdk::new_context();
        ctx.expect().once().returning(|| {
            let mut sdk = MockSdk::default();
            sdk.expect_version().once().returning(|| {
                Ok(qhyccd_rs::SDKVersion {
                    year: 2024,
                    month: 1,
                    day: 1,
                    subday: 0,
                })
            });
            sdk.expect_cameras()
                .once()
                .returning(|| Box::new(Vec::<MockCamera>::new().into_iter()));
            sdk.expect_filter_wheels().once().returning(|| {
                let mut fw = MockFilterWheel::new();
                fw.expect_id().return_const("CFW3-xyz789".to_owned());
                fw.expect_clone().returning(MockFilterWheel::new);
                Box::new(vec![fw].into_iter())
            });
            Ok(sdk)
        });
        let result = ServerBuilder::new().with_port(0).build().await;
        assert!(result.is_ok());
    }
}
