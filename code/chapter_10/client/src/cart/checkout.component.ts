import { CommonModule } from '@angular/common';
import { Component, signal, computed, ViewChild, inject } from '@angular/core';
import { FormBuilder, FormGroup, ReactiveFormsModule, Validators } from '@angular/forms';
import { Router } from '@angular/router';
import { CardInputComponent } from './credit-card.component';
import { NotificationService } from '../services/notification.service';
import { CartService } from '../services/cart.service';
import { OrderService } from '../services/order.service';

@Component({
  selector: 'section.checkout',
  templateUrl: './checkout.component.html',
  styleUrls: ['./checkout.component.scss'],
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule, CardInputComponent],
})
export class CheckoutComponent {
  @ViewChild(CardInputComponent) cardInputComponent?: CardInputComponent;
  private notificationService = inject(NotificationService);
  private cartService = inject(CartService);
  private orderService = inject(OrderService);

  checkoutForm: FormGroup;
  submitted = signal(false);
  isSubmitting = signal(false);

  // Shipping costs for each option
  shippingOptions = [
    { value: 'ups', label: 'UPS', icon: 'bi-truck', cost: 5.99 },
    { value: 'fedex', label: 'FedEx', icon: 'bi-box-seam', cost: 7.99 },
    { value: 'dhl', label: 'DHL', icon: 'bi-mailbox', cost: 9.99 },
  ];

  // Tax rate (10%)
  readonly TAX_RATE = 0.10;

  // Computed signal to track if form is valid
  isFormValid = computed(() => {
    const formValid = this.checkoutForm?.valid ?? false;
    const cardValid = this.cardInputComponent?.isValidCard?.() ?? false;
    return formValid && cardValid;
  });

  constructor(
    private fb: FormBuilder,
    private router: Router,
  ) {
    this.checkoutForm = this.fb.group({
      firstName: ['', [Validators.required, Validators.minLength(2)]],
      lastName: ['', [Validators.required, Validators.minLength(2)]],
      email: ['', [Validators.required, Validators.email]],
      phoneNumber: [
        '',
        [
          Validators.required,
          Validators.pattern(/^[\+]?[(]?[0-9]{1,4}[)]?[-\s\.]?[(]?[0-9]{1,4}[)]?[-\s\.]?[0-9]{1,9}$/),
        ],
      ],
      addressLine1: ['', [Validators.required, Validators.minLength(5)]],
      addressLine2: [''],
      city: ['', [Validators.required, Validators.minLength(2)]],
      state: ['', [Validators.required, Validators.minLength(2)]],
      zipCode: ['', [Validators.required, Validators.pattern(/^[0-9]{5}(-[0-9]{4})?$/)]],
      country: ['US', [Validators.required, Validators.minLength(2)]],
      shippingOption: ['', Validators.required],
      agreeToTerms: [false, Validators.requiredTrue],
    });
  }

  onSubmit(): void {
    this.submitted.set(true);

    // Mark all fields as touched to show validation errors
    Object.keys(this.checkoutForm.controls).forEach((key) => {
      this.checkoutForm.get(key)?.markAsTouched();
    });

    // Mark card input as touched
    if (this.cardInputComponent) {
      this.cardInputComponent.markAsTouched();
    }

    // Check if form and card are valid
    if (!this.isFormValid()) {
      this.notificationService.show('Please fix all errors before submitting', 'danger');
      return;
    }

    // Check if cart is empty
    if (this.cartService.isEmpty()) {
      this.notificationService.show('Your cart is empty', 'danger');
      return;
    }

    // Prevent double submission
    if (this.isSubmitting()) {
      return;
    }

    this.isSubmitting.set(true);

    // Build order request
    const formValue = this.checkoutForm.value;
    const cartItems = this.cartService.cartItems();

    // Build shipping address
    const shippingAddress: CreateShippingAddressRequest = {
      first_name: formValue.firstName,
      last_name: formValue.lastName,
      address_line1: formValue.addressLine1,
      address_line2: formValue.addressLine2 || null,
      city: formValue.city,
      state: formValue.state,
      postal_code: formValue.zipCode,
      country: formValue.country,
      phone: formValue.phoneNumber,
    };

    // Build order items with complete product details
    const items: CreateOrderItemRequest[] = cartItems.map(item => ({
      product_uuid: item.product.eid,
      product_name: item.product.product_name,
      product_sku: null,
      quantity: item.quantity,
      unit_price: item.product.final_price,
    }));

    // Build payment info
    const payment: CreatePaymentRequest = {
      payment_method: 'credit_card',
      card_last4: this.cardInputComponent?.getCardLast4() || null,
      card_brand: this.cardInputComponent?.getCardBrand() || null,
    };

    // Build complete order request
    const orderRequest: CreateOrderRequest = {
      customer_email: formValue.email,
      customer_phone: formValue.phoneNumber,
      shipping_address: shippingAddress,
      items: items,
      payment: payment,
    };

    // Submit order
    this.orderService.createOrder(orderRequest).subscribe({
      next: (response) => {
        this.isSubmitting.set(false);

        if (response.success) {
          // Clear cart on success
          this.cartService.clearCart();

          // Show success notification
          this.notificationService.show('Order placed successfully! Redirecting to products...', 'success');

          // Navigate to products page
          setTimeout(() => {
            this.router.navigate(['/products']);
          }, 1500);
        } else {
          // Handle error response
          const errorMessage = response.error || 'Failed to place order';
          this.notificationService.show(errorMessage, 'danger');
        }
      },
      error: (error) => {
        this.isSubmitting.set(false);

        // Handle HTTP errors
        const errorMessage = error.error?.error || error.message || 'An error occurred while placing the order';
        this.notificationService.show(errorMessage, 'danger');
      }
    });
  }

  // Helper methods to check field validity
  isFieldInvalid(fieldName: string): boolean {
    const field = this.checkoutForm.get(fieldName);
    return !!(field && field.invalid && (field.touched || this.submitted()));
  }

  getFieldError(fieldName: string): string {
    const field = this.checkoutForm.get(fieldName);
    if (!field || !field.errors) return '';

    if (field.errors['required']) return `${this.getFieldLabel(fieldName)} is required`;
    if (field.errors['email']) return 'Please enter a valid email address';
    if (field.errors['minlength']) return `${this.getFieldLabel(fieldName)} is too short`;
    if (field.errors['pattern']) {
      if (fieldName === 'phoneNumber') return 'Please enter a valid phone number';
      if (fieldName === 'zipCode') return 'Please enter a valid ZIP code (e.g., 12345 or 12345-6789)';
    }

    return 'Invalid value';
  }

  private getFieldLabel(fieldName: string): string {
    const labels: { [key: string]: string } = {
      firstName: 'First name',
      lastName: 'Last name',
      email: 'Email',
      phoneNumber: 'Phone number',
      addressLine1: 'Address line 1',
      addressLine2: 'Address line 2',
      zipCode: 'ZIP code',
      city: 'City',
      state: 'State/Province',
      country: 'Country',
      shippingOption: 'Shipping option',
      agreeToTerms: 'Agreement to terms',
    };
    return labels[fieldName] || fieldName;
  }
}
