use std::future::Future;
use std::time::Duration;

use log::debug;

use crate::error::AppError;

pub(super) async fn run_with_timeout<T, E, F, M>(
    timeout: Duration,
    timeout_operation: &'static str,
    future: F,
    map_error: M,
) -> Result<T, AppError>
where
    F: Future<Output = Result<T, E>>,
    M: FnOnce(E) -> AppError,
{
    match tokio::time::timeout(timeout, future).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(map_error(error)),
        Err(_) => Err(AppError::timeout(timeout_operation, timeout.as_secs())),
    }
}

pub(super) async fn retry_with_delays<T, Op, Fut>(
    operation_name: &'static str,
    retry_delays_secs: &[u64],
    mut operation: Op,
) -> Result<T, AppError>
where
    Op: FnMut() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let mut last_err: Option<AppError> = None;

    for (attempt, &delay_secs) in retry_delays_secs.iter().enumerate() {
        if delay_secs > 0 {
            tokio::time::sleep(Duration::from_secs(delay_secs)).await;
        }

        match operation().await {
            Ok(value) => return Ok(value),
            Err(error) => {
                debug!(
                    "{} attempt {} failed: {}",
                    operation_name,
                    attempt + 1,
                    error
                );
                last_err = Some(error);
            }
        }
    }

    match last_err {
        Some(error) => Err(error),
        None => Err(AppError::operation_failed(
            operation_name,
            "no attempts configured",
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Duration;

    use super::{retry_with_delays, run_with_timeout};
    use crate::error::{AppError, AppErrorDetail};

    #[tokio::test]
    async fn run_with_timeout_returns_inner_success_value() {
        let result = run_with_timeout(
            Duration::from_secs(1),
            "fetch",
            async { Ok::<_, &'static str>(42) },
            |_| AppError::operation_failed("fetch", "mapped"),
        )
        .await
        .expect("success result should pass through");

        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn run_with_timeout_maps_operation_error() {
        let result = run_with_timeout(
            Duration::from_secs(1),
            "fetch",
            async { Err::<(), _>("boom") },
            |error| AppError::operation_failed("fetch", error),
        )
        .await;

        assert_eq!(
            result,
            Err(AppError::OperationFailed {
                operation: "fetch",
                details: AppErrorDetail::from("boom")
            })
        );
    }

    #[tokio::test]
    async fn run_with_timeout_returns_timeout_error() {
        let result = run_with_timeout(
            Duration::from_millis(10),
            "install",
            async {
                tokio::time::sleep(Duration::from_secs(2)).await;
                Ok::<_, AppError>(())
            },
            |error| error,
        )
        .await;

        assert_eq!(
            result,
            Err(AppError::Timeout {
                operation: "install",
                seconds: 0,
            })
        );
    }

    #[tokio::test]
    async fn retry_with_delays_retries_until_success() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_op = Arc::clone(&attempts);

        let result = retry_with_delays("load", &[0, 0, 0], move || {
            let attempts_for_this_try = Arc::clone(&attempts_for_op);
            async move {
                let attempt = attempts_for_this_try.fetch_add(1, Ordering::SeqCst);
                if attempt == 0 {
                    Err(AppError::operation_failed("load", "first attempt failed"))
                } else {
                    Ok("ok")
                }
            }
        })
        .await
        .expect("second attempt should succeed");

        assert_eq!(result, "ok");
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn retry_with_delays_returns_last_error_when_all_attempts_fail() {
        let result = retry_with_delays("load", &[0, 0], || async {
            Err::<(), _>(AppError::operation_failed("load", "still failing"))
        })
        .await;

        assert_eq!(
            result,
            Err(AppError::OperationFailed {
                operation: "load",
                details: AppErrorDetail::from("still failing")
            })
        );
    }

    #[tokio::test]
    async fn retry_with_delays_with_no_attempts_returns_structured_error() {
        let result = retry_with_delays("load", &[], || async { Ok::<_, AppError>(()) }).await;

        assert_eq!(
            result,
            Err(AppError::OperationFailed {
                operation: "load",
                details: AppErrorDetail::from("no attempts configured")
            })
        );
    }
}
