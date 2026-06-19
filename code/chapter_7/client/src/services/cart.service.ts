import { Injectable, signal, computed } from '@angular/core';

export interface CartItem {
  product: ProductCard;
  quantity: number;
  addedAt: Date;
}

@Injectable({
  providedIn: 'root'
})
export class CartService {
  // Private signal for cart items
  private readonly _cartItems = signal<CartItem[]>([]);

  // Public readonly signals
  readonly cartItems = this._cartItems.asReadonly();
  
  // Computed signals for derived state
  readonly totalItems = computed(() => 
    this._cartItems().reduce((total, item) => total + item.quantity, 0)
  );
  
  readonly totalPrice = computed(() => 
    this._cartItems().reduce((total, item) => 
      total + (parseFloat(item.product.final_price) * item.quantity), 0
    )
  );
  
  readonly isEmpty = computed(() => this._cartItems().length === 0);

  // Private helper methods
  private findItemIndex(productId: string): number {
    return this._cartItems().findIndex(item => item.product.eid === productId);
  }

  private findItem(productId: string): CartItem | undefined {
    return this._cartItems().find(item => item.product.eid === productId);
  }

  private updateItems(updater: (items: CartItem[]) => CartItem[]): void {
    this._cartItems.set(updater([...this._cartItems()]));
  }

  // Public methods
  addToCart(product: ProductCard, quantity: number = 1): void {
    const existingIndex = this.findItemIndex(product.eid);
    
    if (existingIndex >= 0) {
      // Update existing item
      this.updateItems(items => {
        items[existingIndex].quantity += quantity;
        return items;
      });
    } else {
      // Add new item
      this.updateItems(items => [
        ...items,
        { product, quantity, addedAt: new Date() }
      ]);
    }
  }

  removeFromCart(productId: string): void {
    this.updateItems(items => items.filter(item => item.product.eid !== productId));
  }

  updateQuantity(productId: string, quantity: number): void {
    if (quantity <= 0) {
      this.removeFromCart(productId);
      return;
    }

    this.updateItems(items => 
      items.map(item => 
        item.product.eid === productId 
          ? { ...item, quantity }
          : item
      )
    );
  }

  increaseQuantity(productId: string): void {
    const item = this.findItem(productId);
    if (item) {
      this.updateQuantity(productId, item.quantity + 1);
    }
  }

  decreaseQuantity(productId: string): void {
    const item = this.findItem(productId);
    if (item) {
      this.updateQuantity(productId, item.quantity - 1);
    }
  }

  clearCart(): void {
    this._cartItems.set([]);
  }

  // Query methods
  getItem(productId: string): CartItem | undefined {
    return this.findItem(productId);
  }

  hasItem(productId: string): boolean {
    return this.findItemIndex(productId) >= 0;
  }
}