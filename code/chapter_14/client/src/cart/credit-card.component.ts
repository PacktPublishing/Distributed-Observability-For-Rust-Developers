import { CommonModule } from '@angular/common';
import { Component, computed, signal } from '@angular/core';

const SPECIAL_KEYS = ['Backspace', 'Tab', 'End', 'Home', 'ArrowLeft', 'ArrowRight', 'Delete', 'Enter'];
@Component({
  selector: 'div.card-input',
  standalone: true,
  template: `
    <input
      type="text"
      maxlength="19"
      placeholder="XXXX-XXXX-XXXX-XXXX"
      [value]="format()"
      (input)="handleInput($event)"
      inputmode="numeric"
      class="form-control"
      [class.is-invalid]="showValidation() && !isValidCard()"
      [class.is-valid]="showValidation() && isValidCard()"
      (keydown)="validKey($event) || $event.preventDefault()"
      (blur)="onTouched()"
      autocomplete="off"
    />

    <!-- Feedback Icon Container (Uses Bootstrap Icons) -->
    @if (showValidation() && !isValidCard()) {
      <div class="invalid-feedback d-block">
        <i class="bi bi-exclamation-circle me-1"></i>
        Invalid card number
      </div>
    }
  `,
  styles: `
    input {
      font-size: .85rem;
    }
  `,
  imports: [CommonModule],
})
export class CardInputComponent {
  private rawDigits = signal('');
  private touched = signal(false);

  // --- Computed Signals for State Management ---

  // Formats the raw digits with hyphens (e.g., "1234-5678-...")
  format = computed(() => {
    const digits = this.rawDigits();
    // Trim to maximum 16 digits and insert hyphens every 4 characters
    return (
      digits
        .substring(0, 16)
        .match(/.{1,4}/g)
        ?.join('-') || ''
    );
  });

  // True if the input has exactly 16 clean digits
  isValidCard = computed(() => this.rawDigits().length === 16);
  
  // Show validation feedback when touched and has input
  showValidation = computed(() => this.touched() && this.rawDigits().length > 0);

  /**
   * Handles the input event, cleans the value, and updates the signal.
   * @param event The native input event from the template.
   */
  handleInput(event: Event): void {
    const inputEle = event.target as HTMLInputElement;
    const value = inputEle.value;
    const cleaned = value.replace(/\D/g, '');
    const limitedDigits = cleaned.substring(0, 16);
    this.rawDigits.set(limitedDigits);
  }

  validKey(event: KeyboardEvent): boolean {
    const { key } = event;
    return /^\d$/.test(key) || SPECIAL_KEYS.includes(key);
  }
  
  onTouched(): void {
    this.touched.set(true);
  }
  
  // Public method to mark as touched from parent component
  markAsTouched(): void {
    this.touched.set(true);
  }

  // Get raw card number (for extracting last 4 digits and detecting brand)
  getCardNumber(): string {
    return this.rawDigits();
  }

  // Get last 4 digits of card
  getCardLast4(): string {
    const digits = this.rawDigits();
    return digits.length >= 4 ? digits.slice(-4) : '';
  }

  // Detect card brand from card number
  getCardBrand(): string {
    const digits = this.rawDigits();
    if (digits.length === 0) return '';

    const firstDigit = digits[0];
    if (firstDigit === '4') return 'Visa';
    if (firstDigit === '5') return 'Mastercard';
    if (firstDigit === '3') return 'American Express';
    if (firstDigit === '6') return 'Discover';

    return 'Unknown';
  }
}
