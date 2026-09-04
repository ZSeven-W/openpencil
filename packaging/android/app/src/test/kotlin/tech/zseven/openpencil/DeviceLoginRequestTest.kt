package tech.zseven.openpencil

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class DeviceLoginRequestTest {
    @Test
    fun canonicalVerificationUrlsParse() {
        val parsed = DeviceLoginRequest.parse("https://sso.zseven.cn/login?device_pairing=pair-1")
        assertEquals("https://sso.zseven.cn", parsed?.origin)
        assertEquals("pair-1", parsed?.pairingId)

        val withPort = DeviceLoginRequest.parse(
            "https://sso.example.test:8443/login?device_pairing=x",
        )
        assertEquals("https://sso.example.test:8443", withPort?.origin)

        val extraQuery = DeviceLoginRequest.parse(
            "https://sso.zseven.tech/login?utm=app&device_pairing=abc",
        )
        assertEquals("abc", extraQuery?.pairingId)
    }

    @Test
    fun nonPairingAndNonHttpsUrlsAreRejected() {
        assertNull(DeviceLoginRequest.parse("https://sso.zseven.cn/login"))
        assertNull(DeviceLoginRequest.parse("https://sso.zseven.cn/login?device_pairing="))
        assertNull(DeviceLoginRequest.parse("http://sso.zseven.cn/login?device_pairing=x"))
        assertNull(DeviceLoginRequest.parse("https://user:pw@sso.zseven.cn/login?device_pairing=x"))
        assertNull(DeviceLoginRequest.parse("not a url"))
    }

    @Test
    fun accountSnapshotDecodesEngineJson() {
        val signedIn = AccountSnapshot.parse(
            """{"signed_in":true,"display_name":"Fini","username":"fini",""" +
                """"primary_email":"fini@example.test","avatar_url":null,"device_id":"d1"}""",
        )
        assertEquals(true, signedIn?.signedIn)
        assertEquals("Fini", signedIn?.displayName)
        assertEquals("fini@example.test", signedIn?.primaryEmail)

        val signedOut = AccountSnapshot.parse("""{"signed_in":false}""")
        assertEquals(false, signedOut?.signedIn)
        assertNull(signedOut?.displayName)

        assertNull(AccountSnapshot.parse("not json"))
    }

    @Test
    fun regionalProviderListsParse() {
        val global = SsoProviderList.parse(
            """{"providers":[
                {"id":"apple","display_name":"Apple","icon":"apple","channel":"web_mobile","start_url":"/x"},
                {"id":"github","display_name":"GitHub"},
                {"id":"google","display_name":"Google"}
            ]}""",
        )
        assertEquals(listOf("Apple", "GitHub", "Google"), global.map { it.displayName })

        val mainland = SsoProviderList.parse(
            """{"providers":[
                {"id":"wechat","display_name":"微信"},
                {"id":"alipay","display_name":"支付宝"},
                {"id":"douyin","display_name":"抖音"},
                {"id":"","display_name":"broken"},
                {"id":"nameless"}
            ]}""",
        )
        assertEquals(listOf("wechat", "alipay", "douyin"), mainland.map { it.id })

        assertEquals(emptyList<SsoProviderEntry>(), SsoProviderList.parse("not json"))
        assertEquals(emptyList<SsoProviderEntry>(), SsoProviderList.parse("{}"))
    }
}
