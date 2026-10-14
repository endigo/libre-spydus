package com.librespydus.app.feature.profile

import android.Manifest
import android.content.ActivityNotFoundException
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Copy
import com.librespydus.app.core.NotificationAccess
import com.librespydus.app.feature.onboarding.AccountCard
import com.librespydus.app.feature.onboarding.AccountDivider
import com.librespydus.app.feature.onboarding.AccountSectionHeader
import com.librespydus.app.ui.components.SecondaryPillButton
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import kotlinx.coroutines.launch

private enum class AlertSwitch { READY, DUE }

/**
 * Profile → Notifications (signed in only): "Ready to collect" and "Due-date reminders", both off
 * by default and app-wide. Turning one on first needs notifications allowed: Android 13+ asks for
 * `POST_NOTIFICATIONS` (older versions have no prompt). When they're refused or off in Settings,
 * the switch stays off and a line with "Open Settings" shows under the section.
 */
@Composable
internal fun NotificationsSection(appState: AppState) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val readyOn by appState.alerts.readyEnabled.collectAsStateWithLifecycle()
    val dueOn by appState.alerts.dueEnabled.collectAsStateWithLifecycle()
    // The switch waiting for the permission answer.
    var asking by rememberSaveable { mutableStateOf<AlertSwitch?>(null) }
    // Turning a switch on was refused (cleared once notifications are allowed again).
    var refused by rememberSaveable { mutableStateOf(false) }
    var allowed by remember { mutableStateOf(NotificationAccess.canPost(context)) }

    // Back from Settings (or the permission dialog): notifications may have been turned on or off.
    LifecycleResumeEffect(Unit) {
        allowed = NotificationAccess.canPost(context)
        if (allowed) refused = false
        onPauseOrDispose { }
    }

    fun set(switch: AlertSwitch, on: Boolean) {
        scope.launch {
            when (switch) {
                AlertSwitch.READY -> appState.setReadyAlerts(on)
                AlertSwitch.DUE -> appState.setDueReminders(on)
            }
        }
    }

    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        val switch = asking
        asking = null
        allowed = NotificationAccess.canPost(context)
        if (granted && allowed) {
            refused = false
            switch?.let { set(it, true) }
        } else {
            refused = true
        }
    }

    fun toggle(switch: AlertSwitch, on: Boolean) {
        if (!on) return set(switch, false)
        when (NotificationAccess.step(context)) {
            NotificationAccess.Step.ALLOWED -> {
                refused = false
                set(switch, true)
            }
            NotificationAccess.Step.REQUEST -> {
                asking = switch
                permission.launch(Manifest.permission.POST_NOTIFICATIONS)
            }
            NotificationAccess.Step.BLOCKED -> refused = true
        }
    }

    Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
        AccountSectionHeader(Copy.Notifications.section)
        AccountCard {
            AlertSwitchRow(Copy.Notifications.readyTitle, Copy.Notifications.readySubtitle, readyOn) { toggle(AlertSwitch.READY, it) }
            AccountDivider()
            AlertSwitchRow(Copy.Notifications.dueTitle, Copy.Notifications.dueSubtitle, dueOn) { toggle(AlertSwitch.DUE, it) }
        }
        if (refused || ((readyOn || dueOn) && !allowed)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    Copy.Notifications.blocked,
                    style = LsType.caption,
                    color = LsColors.warning,
                    modifier = Modifier.weight(1f),
                )
                Spacer(Modifier.width(LsSpacing.m))
                SecondaryPillButton(
                    Copy.Notifications.openSettings,
                    onClick = {
                        try {
                            context.startActivity(NotificationAccess.settingsIntent(context))
                        } catch (_: ActivityNotFoundException) {
                        }
                    },
                )
            }
        }
    }
}

/** A switch row: title, a line saying what it does, the whole row toggles. Also Profile's fingerprint-or-face switch. */
@Composable
internal fun AlertSwitchRow(title: String, subtitle: String, checked: Boolean, onChange: (Boolean) -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .toggleable(value = checked, role = Role.Switch, onValueChange = onChange),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(title, style = LsType.bodyStrong, color = LsColors.textPrimary)
            Text(subtitle, style = LsType.caption, color = LsColors.textSecondary)
        }
        Spacer(Modifier.width(LsSpacing.m))
        Switch(
            checked = checked,
            // The whole row toggles (one TalkBack target).
            onCheckedChange = null,
            colors = SwitchDefaults.colors(
                checkedThumbColor = LsColors.onAccent,
                checkedTrackColor = LsColors.accent,
                checkedBorderColor = LsColors.accent,
                uncheckedThumbColor = LsColors.textSecondary,
                uncheckedTrackColor = LsColors.field,
                uncheckedBorderColor = LsColors.divider,
            ),
        )
    }
}
