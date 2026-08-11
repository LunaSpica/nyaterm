use crate::RdpCertificatePolicy;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CertificateDecision {
    Accept,
    AcceptAndRemember,
    Prompt,
    Reject,
}

pub fn evaluate_certificate(
    policy: RdpCertificatePolicy,
    remembered_fingerprint: Option<&str>,
    presented_fingerprint: &str,
) -> CertificateDecision {
    let known_match = remembered_fingerprint == Some(presented_fingerprint);
    let changed = remembered_fingerprint.is_some() && !known_match;
    match policy {
        RdpCertificatePolicy::Insecure => CertificateDecision::Accept,
        RdpCertificatePolicy::TrustOnFirstUse if known_match => CertificateDecision::Accept,
        RdpCertificatePolicy::TrustOnFirstUse if changed => CertificateDecision::Reject,
        RdpCertificatePolicy::TrustOnFirstUse => CertificateDecision::AcceptAndRemember,
        RdpCertificatePolicy::Strict | RdpCertificatePolicy::RejectChanged if known_match => {
            CertificateDecision::Accept
        }
        RdpCertificatePolicy::Strict | RdpCertificatePolicy::RejectChanged => {
            CertificateDecision::Reject
        }
        RdpCertificatePolicy::Prompt if known_match => CertificateDecision::Accept,
        RdpCertificatePolicy::Prompt => CertificateDecision::Prompt,
    }
}

#[cfg(test)]
mod tests {
    use crate::{CertificateDecision, RdpCertificatePolicy, evaluate_certificate};

    #[test]
    fn enforces_certificate_policies() {
        assert_eq!(
            evaluate_certificate(RdpCertificatePolicy::TrustOnFirstUse, None, "a"),
            CertificateDecision::AcceptAndRemember
        );
        assert_eq!(
            evaluate_certificate(RdpCertificatePolicy::TrustOnFirstUse, Some("a"), "b"),
            CertificateDecision::Reject
        );
        assert_eq!(
            evaluate_certificate(RdpCertificatePolicy::Strict, Some("a"), "a"),
            CertificateDecision::Accept
        );
        assert_eq!(
            evaluate_certificate(RdpCertificatePolicy::Strict, None, "a"),
            CertificateDecision::Reject
        );
        assert_eq!(
            evaluate_certificate(RdpCertificatePolicy::Prompt, None, "a"),
            CertificateDecision::Prompt
        );
        assert_eq!(
            evaluate_certificate(RdpCertificatePolicy::Insecure, Some("a"), "b"),
            CertificateDecision::Accept
        );
    }
}
