package com.librespydus.app.feature.borrowed

import com.librespydus.app.core.Formatting
import uniffi.librespydus_core.EditOptions
import uniffi.librespydus_core.EditOverrides
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.Locale

/**
 * One of the edit form's dates ("Delay until" = SDTE, "Not required after" = XDTE). [original] is
 * the form's value as the library wrote it ("2026-11-01"); null when unset.
 */
data class EditDateField(val original: String?, val isOn: Boolean, val date: LocalDate) {
    /** The new value to send, or null when it's off or unchanged. */
    fun changedValue(): String? {
        if (!isOn) return null
        val value = EditDraft.formValue(date, like = original)
        return value.takeIf { it != original }
    }

    /** The form had a date and it was switched off. */
    val isCleared: Boolean get() = !isOn && original != null

    val hasChanges: Boolean get() = isCleared || changedValue() != null

    companion object {
        fun of(original: String?, defaultDate: LocalDate): EditDateField {
            val value = original?.trim()?.takeIf { it.isNotEmpty() }
            val parsed = Formatting.parseDueDate(value)
            return EditDateField(value, isOn = parsed != null, date = parsed ?: defaultDate)
        }
    }
}

/**
 * The edit sheet's form state and the [EditOverrides] it sends. Mirrors the reservation form:
 * pickup branch, "Delay until" and "Not required after". A date is sent only when it changed;
 * switching off (clearing) a date the form had sets the matching clear flag.
 */
data class EditDraft(
    /** Selected `PickupChoice.code`; null when the form offers no branches (digital holds). */
    val pickup: String?,
    val initialPickup: String?,
    val delay: EditDateField,
    val expiry: EditDateField,
) {
    val hasChanges: Boolean get() = pickup != initialPickup || delay.hasChanges || expiry.hasChanges

    /**
     * What `editReservation` sends. The pickup is always the one shown (the form's own value when
     * unchanged), like the web app.
     */
    fun overrides(): EditOverrides = EditOverrides(
        pickup = pickup,
        delayUntil = delay.changedValue(),
        notRequiredAfter = expiry.changedValue(),
        clearDelay = delay.isCleared,
        clearExpiry = expiry.isCleared,
    )

    companion object {
        /**
         * From the form. The pickup starts on the form's selection, else the branch the
         * reservation lists ([currentPickup]), else the first choice.
         */
        fun from(options: EditOptions, currentPickup: String? = null, today: LocalDate = LocalDate.now()): EditDraft {
            val choices = options.pickupChoices
            val selected = options.pickupSelected?.let { code -> choices.firstOrNull { it.code == code }?.code }
            val listed = currentPickup?.let { name -> choices.firstOrNull { it.name.equals(name.trim(), ignoreCase = true) }?.code }
            val pickup = selected ?: listed ?: choices.firstOrNull()?.code
            return EditDraft(
                pickup = pickup,
                initialPickup = pickup,
                delay = EditDateField.of(options.delayUntil, today.plusDays(7)),
                expiry = EditDateField.of(options.notRequiredAfter, today.plusMonths(3)),
            )
        }

        /**
         * [date] in the form's own format: "dd/MM/yyyy" when the form wrote dates that way, else
         * the HTML date input's "yyyy-MM-dd" (what Spydus's edit form uses). Always Gregorian.
         */
        fun formValue(date: LocalDate, like: String?): String {
            val pattern = if (like?.contains('/') == true) "dd/MM/uuuu" else "uuuu-MM-dd"
            return DateTimeFormatter.ofPattern(pattern, Locale.ROOT).format(date)
        }
    }
}
