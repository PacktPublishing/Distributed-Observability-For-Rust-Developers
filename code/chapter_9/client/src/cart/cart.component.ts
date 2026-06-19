import { Component, inject, signal } from '@angular/core';
import { CartService } from '../services/cart.service';
import { CommonModule } from '@angular/common';
import { Router, RouterLink } from '@angular/router';

@Component({
  selector: 'aside.cart',
  templateUrl: './cart.component.html',
  styleUrls: ['./cart.component.scss'],
  standalone: true,
  imports: [CommonModule, RouterLink],
})
export class CartComponent {
  private router = inject(Router);
  readonly cartService = inject(CartService);
  readonly checkoutInProgress = signal(false);
  constructor() {
    this.router.events.pipe().subscribe(() => {
      this.checkoutInProgress.set(this.router.url === '/checkout');
    });
  }

  updateQuantity(item: CartItem, ev: Event): void {
    ev.preventDefault();
    ev.stopPropagation();
    this.cartService.updateQuantity(item.product.eid, item.quantity + 1);
  }

  discount(product: ProductCard): number {
    const initial = parseFloat(product.initial_price || '0');
    const discount = parseFloat(product.discount || '0');
    return initial > 0 ? Math.round((discount / initial) * 100) : 0;
  }

  removeItem(productId: string, ev: Event): void {
    ev.preventDefault();
    ev.stopPropagation();
    this.cartService.removeFromCart(productId);
  }

  clearCart(): void {
    this.cartService.clearCart();
    this.router.navigate(['/products']);
  }
}
