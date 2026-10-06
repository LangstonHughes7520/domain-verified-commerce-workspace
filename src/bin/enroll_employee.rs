use domain_verified_commerce_workspace::infrai_client::InfraiClient;
use domain_verified_commerce_workspace::workspace_join::{enroll_employee, EnrollmentRequest};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("enrollment failed: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let request = EnrollmentRequest {
        domain: required_arg(&mut args, "domain")?,
        proof_name: required_arg(&mut args, "proof_name")?,
        proof_content: required_arg(&mut args, "proof_content")?,
        employee_email: required_arg(&mut args, "employee_email")?,
        employee_name: required_arg(&mut args, "employee_name")?,
        idempotency_key: required_arg(&mut args, "idempotency_key")?,
    };

    let client = InfraiClient::from_env()?;
    let membership = enroll_employee(&client, request).await?;
    println!(
        "joined user {} to {} for verified domain {}",
        membership.user_id, membership.workspace, membership.company_domain
    );
    Ok(())
}

fn required_arg(
    args: &mut impl Iterator<Item = String>,
    name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    args.next()
        .ok_or_else(|| format!("missing argument: {name}").into())
}
