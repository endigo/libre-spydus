package com.librespydus.app.feature.mycard

import android.app.Activity
import androidx.activity.compose.LocalActivity
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.BottomSheetDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.core.AppState
import com.librespydus.app.feature.onboarding.LibraryLogo
import com.librespydus.app.ui.components.CircleIconButton
import com.librespydus.app.ui.components.PrimaryPillButton
import com.librespydus.app.ui.components.SecondaryPillButton
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import kotlinx.coroutines.launch
import uniffi.librespydus_core.CardBarcode
import uniffi.librespydus_core.LibraryInfo

/** The entry form's note under the field (docs/native-apps.md §5.9, as amended). */
const val CARD_NUMBER_NOTE = "Enter the number on your library card or in your library account's Personal details (e.g. CL114D…)."

/**
 * My card (§5.9): the library card as a barcode to scan at the desk or self-checkout, on a navy
 * sheet like the other sheets. Full screen brightness while it is up.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MyCardSheet(appState: AppState, onDismiss: () -> Unit) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val scope = rememberCoroutineScope()
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        containerColor = LsColors.background,
        contentColor = LsColors.textPrimary,
        dragHandle = { BottomSheetDefaults.DragHandle(color = LsColors.textMuted) },
    ) {
        FullBrightness()
        MyCardContent(
            appState = appState,
            onClose = { scope.launch { sheetState.hide() }.invokeOnCompletion { onDismiss() } },
        )
    }
}

@Composable
private fun MyCardContent(appState: AppState, onClose: () -> Unit) {
    val library by appState.library.collectAsStateWithLifecycle()
    val backend by appState.backend.collectAsStateWithLifecycle()
    val accountName by appState.accountName.collectAsStateWithLifecycle()
    val number by appState.cardNumber.collectAsStateWithLifecycle()
    val expires by appState.cardExpires.collectAsStateWithLifecycle()
    val origin = backend?.origin
    var symbology by remember(origin) { mutableStateOf(origin?.let(CardSymbologies.shared::cached)) }
    var editing by rememberSaveable { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(backend) {
        val current = backend ?: return@LaunchedEffect
        symbology = CardSymbologies.shared.load(current)
    }
    val content = remember(number, symbology) { MyCardLogic.content(number, symbology) }

    Column(
        Modifier
            .fillMaxWidth()
            .imePadding()
            .navigationBarsPadding()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = LsSpacing.xl)
            .padding(bottom = LsSpacing.xxl),
        verticalArrangement = Arrangement.spacedBy(LsSpacing.l),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                "My card",
                style = LsType.greeting,
                color = LsColors.textPrimary,
                modifier = Modifier
                    .weight(1f)
                    .semantics { heading() },
            )
            CircleIconButton(
                LsIcons.Close, "Close", onClose,
                size = 36.dp, iconSize = 18.dp, background = LsColors.surface, contentColor = LsColors.textSecondary, shadow = false,
            )
        }

        if (content == CardContent.Entry || editing) {
            CardNumberForm(
                initial = if (editing) number.orEmpty() else "",
                onSave = { entered ->
                    scope.launch {
                        appState.setCardNumber(entered)
                        editing = false
                    }
                },
                onCancel = if (editing && number != null) ({ editing = false }) else null,
            )
        } else {
            LibraryCardView(library, accountName, content, expires)
            Text(
                "Show this at the library desk or self-checkout.",
                style = LsType.body,
                color = LsColors.textSecondary,
                textAlign = TextAlign.Center,
                modifier = Modifier.fillMaxWidth(),
            )
            SecondaryPillButton(
                "Edit card number",
                onClick = { editing = true },
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}

/** The white card (white in any theme: scanners need black on white). */
@Composable
private fun LibraryCardView(library: LibraryInfo?, accountName: String?, content: CardContent, expires: String?) {
    val shape = RoundedCornerShape(20.dp)
    Column(
        Modifier
            .fillMaxWidth()
            .shadow(12.dp, shape)
            .clip(shape)
            .background(Color.White)
            .padding(vertical = LsSpacing.xl),
        verticalArrangement = Arrangement.spacedBy(LsSpacing.l),
    ) {
        Row(
            Modifier.padding(horizontal = LsSpacing.xl),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(LsSpacing.m),
        ) {
            if (library != null) LibraryLogo(library, size = 40.dp)
            Text(library?.name ?: "Library card", style = LsType.bodyStrong, color = LsColors.onSheet, modifier = Modifier.weight(1f))
        }
        accountName?.trim()?.takeIf { it.isNotEmpty() }?.let { name ->
            Text(name, style = LsType.title, color = LsColors.onSheet, modifier = Modifier.padding(horizontal = LsSpacing.xl))
        }
        // The bars span the card's full width: the white card around them is the quiet zone too.
        when (content) {
            is CardContent.Barcode -> Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
                BarcodeView(content.barcode, MyCardLogic.barcodeDescription(content.number), Modifier.fillMaxWidth().height(BARCODE_HEIGHT))
                CardNumberText(content.barcode.text)
                ExpiryText(expires)
            }
            is CardContent.Loading -> Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
                Box(
                    Modifier
                        .fillMaxWidth()
                        .height(BARCODE_HEIGHT)
                        .semantics { contentDescription = "Loading barcode" },
                    contentAlignment = Alignment.Center,
                ) {
                    CircularProgressIndicator(color = LsColors.onSheetSecondary, strokeWidth = 3.dp, modifier = Modifier.size(32.dp))
                }
                CardNumberText(content.number)
                ExpiryText(expires)
            }
            is CardContent.Unencodable -> Column(
                Modifier.padding(horizontal = LsSpacing.xl),
                verticalArrangement = Arrangement.spacedBy(LsSpacing.s),
            ) {
                Text(
                    content.number,
                    style = LsType.greeting.copy(fontFamily = FontFamily.Monospace, letterSpacing = 2.sp),
                    color = LsColors.onSheet,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.fillMaxWidth(),
                )
                Text(
                    "This number can't be shown as a barcode.",
                    style = LsType.body,
                    color = LsColors.onSheetSecondary,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
            CardContent.Entry -> Unit
        }
    }
}

@Composable
private fun CardNumberText(text: String) {
    Text(
        text,
        style = LsType.section.copy(fontFamily = FontFamily.Monospace, letterSpacing = 3.sp),
        color = LsColors.onSheet,
        textAlign = TextAlign.Center,
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = LsSpacing.l),
    )
}

/** "Expires 12 Mar 2027" under the number, when the account gives the card's expiry. */
@Composable
private fun ExpiryText(expires: String?) {
    val date = expires?.trim()?.takeIf { it.isNotEmpty() } ?: return
    Text(
        "Expires $date",
        style = LsType.caption,
        color = LsColors.onSheetSecondary,
        textAlign = TextAlign.Center,
        modifier = Modifier.fillMaxWidth(),
    )
}

private val BARCODE_HEIGHT = 100.dp

/**
 * The bars, black on white, each module the same whole number of device pixels ([BarcodeLayout])
 * and drawn at whole-pixel positions, so nothing is scaled or smoothed. One TalkBack element.
 */
@Composable
fun BarcodeView(barcode: CardBarcode, description: String, modifier: Modifier = Modifier) {
    val modules = barcode.modules
    Canvas(modifier.clearAndSetSemantics { contentDescription = description }) {
        drawRect(Color.White)
        val layout = BarcodeLayout.of(modules.size, size.width.toInt())
        val height = size.height.toInt().toFloat()
        for (bar in layout.bars(modules)) {
            drawRect(Color.Black, topLeft = Offset(bar.first.toFloat(), 0f), size = Size((bar.last - bar.first + 1).toFloat(), height))
        }
    }
}

/** "Library card number" + note + Save (disabled while blank; the keyboard's Done saves too). */
@Composable
private fun CardNumberForm(initial: String, onSave: (String) -> Unit, onCancel: (() -> Unit)?) {
    var value by remember { mutableStateOf(TextFieldValue(initial, TextRange(initial.length))) }
    val canSave = value.text.isNotBlank()
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { runCatching { focus.requestFocus() } }

    Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.l)) {
        Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
            Text("Library card number", style = LsType.caption, color = LsColors.textSecondary)
            val shape = RoundedCornerShape(LsRadius.field)
            BasicTextField(
                value = value,
                onValueChange = { value = it },
                singleLine = true,
                textStyle = LsType.body.copy(color = LsColors.textPrimary),
                cursorBrush = SolidColor(LsColors.primary),
                keyboardOptions = KeyboardOptions(
                    capitalization = KeyboardCapitalization.Characters,
                    autoCorrectEnabled = false,
                    keyboardType = KeyboardType.Text,
                    imeAction = ImeAction.Done,
                ),
                keyboardActions = KeyboardActions(onDone = { if (canSave) onSave(value.text) }),
                modifier = Modifier
                    .fillMaxWidth()
                    .focusRequester(focus)
                    .semantics { contentDescription = "Library card number" },
                decorationBox = { inner ->
                    Box(
                        Modifier
                            .fillMaxWidth()
                            .heightIn(min = 52.dp)
                            .clip(shape)
                            .background(LsColors.field)
                            .padding(horizontal = LsSpacing.l, vertical = LsSpacing.m),
                        contentAlignment = Alignment.CenterStart,
                    ) {
                        if (value.text.isEmpty()) Text("Card number", style = LsType.body, color = LsColors.textMuted)
                        inner()
                    }
                },
            )
            Text(CARD_NUMBER_NOTE, style = LsType.caption, color = LsColors.textSecondary)
        }
        PrimaryPillButton("Save", onClick = { onSave(value.text) }, enabled = canSave, modifier = Modifier.fillMaxWidth())
        if (onCancel != null) SecondaryPillButton("Cancel", onClick = onCancel, modifier = Modifier.fillMaxWidth())
    }
}

/**
 * Screen brightness to full while this is in the composition, back to the window's own setting
 * when it leaves or the app leaves the foreground (and up again when it returns).
 */
@Composable
private fun FullBrightness() {
    val activity: Activity = LocalActivity.current ?: return
    DisposableEffect(activity) {
        val window = activity.window
        val boost = BrightnessBoost(
            read = { window.attributes.screenBrightness },
            write = { level -> window.attributes = window.attributes.apply { screenBrightness = level } },
        )
        val lifecycle = (activity as? LifecycleOwner)?.lifecycle
        val observer = LifecycleEventObserver { _, event ->
            when (event) {
                Lifecycle.Event.ON_START -> boost.raise()
                Lifecycle.Event.ON_STOP -> boost.restore()
                else -> Unit
            }
        }
        boost.raise()
        lifecycle?.addObserver(observer)
        onDispose {
            lifecycle?.removeObserver(observer)
            boost.restore()
        }
    }
}
