use crate::infrai_client::{InfraiClient, InfraiError};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrollmentRequest {
    pub domain: String,
    pub proof_name: String,
    pub proof_content: String,
    pub employee_email: String,
    pub employee_name: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMembership {
    pub user_id: String,
    pub workspace: String,
    pub company_domain: String,
}

#[derive(Debug, Error)]
pub enum EnrollmentError {
    #[error("employee email domain must match {expected}")]
    EmailDomainMismatch { expected: String },
    #[error("company domain ownership has not been verified")]
    DomainNotVerified,
    #[error(transparent)]
    Infrai(#[from] InfraiError),
}

pub fn email_belongs_to_domain(email: &str, domain: &str) -> bool {
    email
        .rsplit_once('@')
        .map(|(_, email_domain)| email_domain.eq_ignore_ascii_case(domain))
        .unwrap_or(false)
}

pub async fn enroll_employee(
    client: &InfraiClient,
    request: EnrollmentRequest,
) -> Result<WorkspaceMembership, EnrollmentError> {
    if !email_belongs_to_domain(&request.employee_email, &request.domain) {
        return Err(EnrollmentError::EmailDomainMismatch {
            expected: request.domain,
        });
    }

    let domain = client.add_domain(&request.domain).await?;
    client
        .upsert_txt_record(&domain.zone_id, &request.proof_name, &request.proof_content)
        .await?;

    let verification = client.verify_domain(&request.domain).await?;
    if !verification.verified {
        return Err(EnrollmentError::DomainNotVerified);
    }

    let user = client
        .create_employee(
            &request.employee_email,
            &request.employee_name,
            &request.domain,
            &request.idempotency_key,
        )
        .await?;

    Ok(WorkspaceMembership {
        user_id: user.user_id,
        workspace: "commerce-operations".to_owned(),
        company_domain: request.domain,
    })
}

#[cfg(test)]
mod tests {
    use super::email_belongs_to_domain;

    #[test]
    fn employee_must_use_the_verified_company_domain() {
        assert!(email_belongs_to_domain("ops@shop.example", "shop.example"));
        assert!(email_belongs_to_domain("OPS@SHOP.EXAMPLE", "shop.example"));
        assert!(!email_belongs_to_domain(
            "ops@contractor.example",
            "shop.example"
        ));
        assert!(!email_belongs_to_domain("not-an-email", "shop.example"));
    }
}
