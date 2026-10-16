package com.librespydus.app.feature.borrowed

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.AnchoredDraggableState
import androidx.compose.foundation.gestures.DraggableAnchors
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.anchoredDraggable
import androidx.compose.foundation.gestures.animateTo
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.interaction.DragInteraction
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import com.librespydus.app.core.Copy
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import kotlin.math.roundToInt

/**
 * An action on a Your Books card: revealed by swiping the card toward the leading edge, offered in
 * its long-press menu and as a TalkBack custom action. A reservation's are [ReservationAction], a
 * loan's [LoanAction]; only the ones the library linked are offered.
 */
interface SwipeAction {
    /** The swipe tile's label. */
    val title: String

    /** The long-press menu item and the TalkBack custom action. */
    val fullTitle: String

    /** The swipe tile's colour. */
    val tileColor: Color

    /** The swipe tile's and the menu item's icon. */
    val icon: ImageVector

    /** Shown in the danger colour in the menu. */
    val isDestructive: Boolean get() = false

    /** What the swipe tile does, for TalkBack. */
    val hint: String? get() = null
}

/** A reservation's actions. */
enum class ReservationAction(override val title: String, override val fullTitle: String) : SwipeAction {
    EDIT(Copy.Common.edit, Copy.YourBooks.editReservation),
    CANCEL(Copy.Common.cancel, Copy.YourBooks.cancelReservation),
    ;

    override val tileColor: Color get() = if (this == CANCEL) LsColors.danger else LsColors.accent
    override val icon: ImageVector get() = if (this == CANCEL) LsIcons.Close else ReservationIcons.Edit
    override val isDestructive: Boolean get() = this == CANCEL

    companion object {
        /** In trailing order: Edit, then the destructive Cancel at the card's edge (as on iOS). */
        fun of(row: ReservationRow): List<ReservationAction> = buildList {
            if (row.editHref != null) add(EDIT)
            if (row.cancelHref != null) add(CANCEL)
        }
    }
}

/** A loan's actions (§5.12): Renew, when the library links it. */
enum class LoanAction(override val title: String, override val fullTitle: String) : SwipeAction {
    RENEW(Copy.YourBooks.renew, Copy.YourBooks.renewLoan),
    ;

    override val tileColor: Color get() = LsColors.accent
    override val icon: ImageVector get() = ReservationIcons.Renew
    override val hint: String get() = Copy.YourBooks.renewHint

    companion object {
        fun of(row: LoanRow): List<LoanAction> = if (row.renewHref != null) listOf(RENEW) else emptyList()
    }
}

/** At least one loan can be renewed (shows the "Swipe left on a loan…" hint under the list). */
val List<LoanRow>.canRenew: Boolean get() = any { it.renewHref != null }

/** At least one reservation can be swiped (shows the "Swipe left…" hint under the list). */
val ReservationSections.hasActions: Boolean
    get() = ready.any { ReservationAction.of(it).isNotEmpty() } || pending.any { ReservationAction.of(it).isNotEmpty() }

/**
 * Which card (a reservation, or a loan) is swiped open: at most one. Hoisted to the screen, so
 * opening a card closes the others, and a scroll or a tap elsewhere closes it.
 */
@Stable
class ReservationSwipeController {
    var openId: String? by mutableStateOf(null)
        private set

    fun open(id: String) {
        openId = id
    }

    /** Closes [id], or whichever card is open when null. */
    fun close(id: String? = null) {
        if (id == null || openId == id) openId = null
    }

    /** A tap while a card is open only closes it: true when there was one to close. */
    fun dismiss(): Boolean = (openId != null).also { close() }
}

/** Swipe geometry: one rounded tile per action, each with a gap before it. */
internal object ReservationSwipe {
    val tileWidth = 76.dp
    val gap = LsSpacing.s

    /** How far the card slides to show [actions] tiles. */
    fun revealWidth(actions: Int): Dp = (tileWidth + gap) * actions.coerceAtLeast(0)
}

private enum class SwipeAnchor { Closed, Open }

/**
 * Swipe-to-reveal for a Your Books card. Dragging [content] toward the leading edge snaps it open
 * on rounded [actions] tiles at its trailing side (a reservation's Edit, then Cancel; a loan's
 * Renew); a tile runs [onAction] and closes the card. Only two anchors, so no swipe length runs an
 * action by itself. Horizontal drags only: vertical ones still scroll the list. [surfaceTop] is
 * where the card's rounded surface starts below the cover that rises above it: the tiles line up
 * with the surface, and while the card moves it is clipped to that silhouette. [controller] keeps
 * one card open at a time.
 */
@Composable
fun <A : SwipeAction> ReservationSwipeBox(
    id: String,
    controller: ReservationSwipeController,
    actions: List<A>,
    onAction: (A) -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    surfaceTop: Dp = 0.dp,
    content: @Composable () -> Unit,
) {
    val revealPx = with(LocalDensity.current) { ReservationSwipe.revealWidth(actions.size).toPx() }
    val state = remember(revealPx) {
        AnchoredDraggableState(
            initialValue = SwipeAnchor.Closed,
            anchors = DraggableAnchors {
                SwipeAnchor.Closed at 0f
                if (revealPx > 0f) SwipeAnchor.Open at -revealPx
            },
        )
    }
    val canSwipe = enabled && actions.isNotEmpty()
    val isOpenHere = controller.openId == id
    val drags = remember { MutableInteractionSource() }

    // Heading open makes this the open card (closing any other); heading closed releases it.
    LaunchedEffect(state, id) {
        snapshotFlow { state.targetValue }.collect { target ->
            if (target == SwipeAnchor.Open) controller.open(id) else controller.close(id)
        }
    }
    // Starting to swipe this card closes the open one straight away.
    LaunchedEffect(drags, id) {
        drags.interactions.collect { if (it is DragInteraction.Start && controller.openId != id) controller.close() }
    }
    // Another card opened, the list scrolled, a tap landed elsewhere, or the row became busy.
    LaunchedEffect(state, isOpenHere, canSwipe) {
        if ((!isOpenHere || !canSwipe) && state.targetValue == SwipeAnchor.Open) state.animateTo(SwipeAnchor.Closed)
    }
    DisposableEffect(id) { onDispose { controller.close(id) } }

    val offset by remember(state) { derivedStateOf { state.offset.takeUnless { it.isNaN() } ?: 0f } }
    val revealed by remember(state) { derivedStateOf { offset < -0.5f } }
    val clipShape = remember(surfaceTop) { CardSilhouette(surfaceTop, LsRadius.card) }

    Box(
        modifier
            .fillMaxWidth()
            .graphicsLayer {
                clip = revealed
                shape = clipShape
            }
            .anchoredDraggable(state, Orientation.Horizontal, enabled = canSwipe, interactionSource = drags),
    ) {
        // The tiles follow the card's trailing edge in (never under its rounded corners), and are
        // composed only while it's moved, so TalkBack doesn't find hidden tiles (the card's custom
        // actions cover them).
        if (revealed) {
            Row(
                Modifier
                    .matchParentSize()
                    .padding(top = surfaceTop)
                    .offset { IntOffset((revealPx + offset).roundToInt(), 0) },
                horizontalArrangement = Arrangement.spacedBy(ReservationSwipe.gap, Alignment.End),
            ) {
                actions.forEach { action ->
                    SwipeTile(
                        action,
                        onClick = {
                            controller.close()
                            onAction(action)
                        },
                        modifier = Modifier.width(ReservationSwipe.tileWidth).fillMaxHeight(),
                    )
                }
            }
        }
        // Fills the height a grid row gives it (§6.1); in a list, the card's own height.
        Box(Modifier.fillMaxHeight().offset { IntOffset(offset.roundToInt(), 0) }) { content() }
    }
}

/**
 * Closes the open card when a tap lands anywhere in the list, after the tap itself was handled (a
 * tile still runs its action; a card checks [ReservationSwipeController.dismiss] first). Drags are
 * left alone: scrolling closes it separately, and swiping another card takes over.
 */
fun Modifier.closesReservationSwipeOnTap(controller: ReservationSwipeController): Modifier =
    pointerInput(controller) {
        awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
            if (controller.openId == null) return@awaitEachGesture
            var isTap = true
            do {
                val event = awaitPointerEvent(PointerEventPass.Final)
                val change = event.changes.firstOrNull { it.id == down.id } ?: break
                if ((change.position - down.position).getDistance() > viewConfiguration.touchSlop) isTap = false
            } while (change.pressed)
            if (isTap) controller.close()
        }
    }

@Composable
private fun SwipeTile(action: SwipeAction, onClick: () -> Unit, modifier: Modifier = Modifier) {
    Column(
        modifier
            .clip(RoundedCornerShape(LsRadius.card))
            .background(action.tileColor)
            .clickable(role = Role.Button, onClick = onClick)
            .semantics {
                contentDescription = action.hint?.let { Copy.A11y.labelAndHint(action.fullTitle, it) } ?: action.fullTitle
            }
            .padding(horizontal = LsSpacing.xs),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Icon(action.icon, contentDescription = null, tint = Color.White, modifier = Modifier.size(22.dp))
        Spacer(Modifier.height(LsSpacing.xs))
        Text(
            action.title,
            style = LsType.caption.copy(fontWeight = FontWeight.SemiBold),
            color = Color.White,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.clearAndSetSemantics {},
        )
    }
}

/**
 * The card's outline while it slides: its rounded surface, plus the strip above where the cover
 * rises (square, so the cover isn't nicked at rest).
 */
private class CardSilhouette(private val surfaceTop: Dp, private val radius: Dp) : Shape {
    override fun createOutline(size: Size, layoutDirection: LayoutDirection, density: Density): Outline {
        val top = with(density) { surfaceTop.toPx() }.coerceIn(0f, size.height)
        val corner = with(density) { radius.toPx() }
        val path = Path().apply {
            if (top > 0f) addRect(Rect(0f, 0f, size.width, top))
            addRoundRect(RoundRect(0f, top, size.width, size.height, CornerRadius(corner)))
        }
        return Outline.Generic(path)
    }
}

/** Icons only the card actions use (Material geometry, 24dp grid). */
internal object ReservationIcons {
    val Edit: ImageVector = ImageVector.Builder("edit", 24.dp, 24.dp, 24f, 24f)
        .addPath(
            pathData = addPathNodes(
                "M3,17.25V21h3.75L17.81,9.94l-3.75,-3.75L3,17.25zM20.71,7.04c0.39,-0.39 0.39,-1.02 0,-1.41l-2.34,-2.34c-0.39,-0.39 -1.02,-0.39 -1.41,0l-1.83,1.83 3.75,3.75 1.83,-1.83z",
            ),
            fill = SolidColor(Color.Black),
        )
        .build()

    /** Material "autorenew": a loan's Renew. */
    val Renew: ImageVector = ImageVector.Builder("autorenew", 24.dp, 24.dp, 24f, 24f)
        .addPath(
            pathData = addPathNodes(
                "M12,6v3l4,-4 -4,-4v3c-4.42,0 -8,3.58 -8,8 0,1.57 0.46,3.03 1.24,4.26L6.7,14.8c-0.45,-0.83 -0.7,-1.79 -0.7,-2.8 0,-3.31 2.69,-6 6,-6zM18.76,7.74L17.3,9.2c0.44,0.84 0.7,1.79 0.7,2.8 0,3.31 -2.69,6 -6,6v-3l-4,4 4,4v-3c4.42,0 8,-3.58 8,-8 0,-1.57 -0.46,-3.03 -1.24,-4.26z",
            ),
            fill = SolidColor(Color.Black),
        )
        .build()
}
