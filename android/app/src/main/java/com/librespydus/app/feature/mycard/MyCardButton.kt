package com.librespydus.app.feature.mycard

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.material3.minimumInteractiveComponentSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.unit.dp
import com.librespydus.app.R
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType

/** Home's "My card" capsule (§5.9), right of the greeting while signed in: opens the card sheet. */
@Composable
fun MyCardButton(onClick: () -> Unit, modifier: Modifier = Modifier) {
    Row(
        modifier
            // A 48dp tap target around the 40dp capsule.
            .minimumInteractiveComponentSize()
            .height(40.dp)
            .clip(RoundedCornerShape(LsRadius.pill))
            .background(LsColors.surfaceElevated)
            .clickable(role = Role.Button, onClick = onClick)
            .clearAndSetSemantics {
                contentDescription = "My card"
                role = Role.Button
                onClick { onClick(); true }
            }
            .padding(start = LsSpacing.m, end = LsSpacing.l),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(LsSpacing.s),
    ) {
        Icon(painterResource(R.drawable.ic_barcode), contentDescription = null, tint = LsColors.primary, modifier = Modifier.size(20.dp))
        Text("My card", style = LsType.bodyStrong, color = LsColors.textPrimary, maxLines = 1)
    }
}
