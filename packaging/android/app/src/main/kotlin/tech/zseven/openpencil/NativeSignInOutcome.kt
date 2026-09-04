package tech.zseven.openpencil

/**
 * Result of one native provider-SDK authorization attempt (Douyin / Alipay):
 * an auth code for the SSO native-login exchange, a user cancel that simply
 * returns to the login screen, or a failure surfaced as an inline error.
 */
internal sealed interface NativeSignInOutcome {
    data class Authorized(val authCode: String) : NativeSignInOutcome
    data object Canceled : NativeSignInOutcome
    data object Failed : NativeSignInOutcome
}
