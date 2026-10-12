import 'dart:convert';

/// Borrower identity decoded from the access-token JWT.
/// Claim names mirror `Spydus.Domain.DTO.ClaimKeys`.
class BorrowerClaims {
  final String? borrowerBarcode; // BrwId
  final String? consortiumRecordId; // BrwdIrn  (notification paths)
  final String? localAccountId; // BrwIrn   (device / self-issue paths)
  final String? sessionId; // BrwSessionId
  final String? institutionId; // InstitutionId
  final String? name; // name
  final String? preferredUsername; // preferred_username
  final String? version; // ver

  const BorrowerClaims({
    this.borrowerBarcode,
    this.consortiumRecordId,
    this.localAccountId,
    this.sessionId,
    this.institutionId,
    this.name,
    this.preferredUsername,
    this.version,
  });

  /// Decodes the JWT payload (no signature verification — client-side read only,
  /// same as the original app's claim extraction).
  static BorrowerClaims? fromJwt(String jwt) {
    final parts = jwt.split('.');
    if (parts.length < 2) return null;
    try {
      final payload = utf8.decode(base64Url.decode(base64Url.normalize(parts[1])));
      final map = jsonDecode(payload) as Map<String, dynamic>;
      String? s(String k) => map[k]?.toString();
      return BorrowerClaims(
        borrowerBarcode: s('BrwId'),
        consortiumRecordId: s('BrwdIrn'),
        localAccountId: s('BrwIrn'),
        sessionId: s('BrwSessionId'),
        institutionId: s('InstitutionId'),
        name: s('name'),
        preferredUsername: s('preferred_username'),
        version: s('ver'),
      );
    } catch (_) {
      return null;
    }
  }
}
