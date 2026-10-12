import 'dart:convert';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http_mock_adapter/http_mock_adapter.dart';
import 'package:spydus_clone/src/api/identity_client.dart';
import 'package:spydus_clone/src/auth/auth_service.dart';
import 'package:spydus_clone/src/auth/jwt_claims.dart';
import 'package:spydus_clone/src/auth/token_store.dart';
import 'package:spydus_clone/src/config/app_config.dart';

String _jwt(Map<String, dynamic> claims) {
  String seg(Map<String, dynamic> m) =>
      base64Url.encode(utf8.encode(jsonEncode(m))).replaceAll('=', '');
  return '${seg({'alg': 'none'})}.${seg(claims)}.';
}

void main() {
  group('AuthService.buildAuthorizeUrl', () {
    final svc = AuthService(
      identity: IdentityClient(Dio()),
      tokenStore: InMemoryTokenStore(),
    );

    test('matches the recovered OAuth contract', () {
      final uri = svc.buildAuthorizeUrl();
      expect(uri.scheme, 'https');
      expect(uri.host, 'idp.spydus.com');
      expect(uri.path, '/openid/authorize');
      expect(uri.queryParameters['client_id'], 'spydusmobile');
      expect(uri.queryParameters['redirect_uri'], 'spyduslibrary://auth');
      expect(uri.queryParameters['response_type'], 'code');
      expect(uri.queryParameters['scope'], 'openid profile mobile');
      expect(uri.queryParameters.containsKey('prompt'), isFalse);
    });

    test('forceInteractive adds prompt=login', () {
      final uri = svc.buildAuthorizeUrl(forceInteractive: true);
      expect(uri.queryParameters['prompt'], 'login');
    });
  });

  group('Token lifecycle', () {
    test('exchangeCode stores tokens (snake_case response)', () async {
      final dio = Dio(BaseOptions(baseUrl: AppConfig.identityBaseUrl));
      final adapter = DioAdapter(dio: dio);
      adapter.onGet(
        '/openid/token',
        (s) => s.reply(200, {'access_token': 'AT', 'refresh_token': 'RT'}),
        queryParameters: {
          'code': 'CODE',
          'grant_type': 'authorization_code',
          'client_id': 'spydusmobile',
        },
      );
      final store = InMemoryTokenStore();
      final svc = AuthService(identity: IdentityClient(dio), tokenStore: store);

      final tokens = await svc.exchangeCode('CODE');
      expect(tokens.accessToken, 'AT');
      expect((await store.read())?.refreshToken, 'RT');
    });

    test('refresh stores new tokens (camelCase response)', () async {
      final dio = Dio(BaseOptions(baseUrl: AppConfig.identityBaseUrl));
      final adapter = DioAdapter(dio: dio);
      adapter.onGet(
        '/openid/refreshtoken',
        (s) => s.reply(200, {'accessToken': 'AT2', 'refreshToken': 'RT2'}),
        queryParameters: {'refreshToken': 'RT', 'clientId': 'spydusmobile'},
      );
      final store = InMemoryTokenStore(const AuthTokens(accessToken: 'AT', refreshToken: 'RT'));
      final svc = AuthService(identity: IdentityClient(dio), tokenStore: store);

      final tokens = await svc.refresh(const AuthTokens(accessToken: 'AT', refreshToken: 'RT'));
      expect(tokens?.accessToken, 'AT2');
      expect((await store.read())?.accessToken, 'AT2');
    });
  });

  group('BorrowerClaims', () {
    test('decodes consortium/local ids from JWT', () async {
      final token = _jwt({'BrwdIrn': '123', 'BrwIrn': '456', 'name': 'Reader'});
      final store = InMemoryTokenStore(AuthTokens(accessToken: token, refreshToken: 'RT'));
      final svc = AuthService(identity: IdentityClient(Dio()), tokenStore: store);

      final claims = await svc.currentBorrower();
      expect(claims?.consortiumRecordId, '123');
      expect(claims?.localAccountId, '456');
      expect(claims?.name, 'Reader');
      expect(BorrowerClaims.fromJwt('not-a-jwt'), isNull);
    });
  });
}
