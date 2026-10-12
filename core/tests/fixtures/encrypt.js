function rsaEncrypt(val) {
     // Create the encryption object and set the key.
    var crypt = new JSEncrypt();
    crypt.setPublicKey('-----BEGIN PUBLIC KEY-----\
MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQC82Gt72tYMWhZsxs79+VFHgFP2\
kxeFvCuD/eg3NJOXAG9MMMDGzdQCCcbHDUXSHpwRE4ghacQrbr//JfBv+X5ZOeWU\
fnw3GUdlC+1CiPLlGzK/5oAl/hoKwaVt8K6MKqZphFsm5ftJbnc8xGNrbVG+AXB0\
bIAHZuvUILyfQV0+RwIDAQAB\
-----END PUBLIC KEY-----\
'); //You can use also setPrivateKey and setPublicKey, they are both alias to setKey

	return crypt.encrypt(val);	
}