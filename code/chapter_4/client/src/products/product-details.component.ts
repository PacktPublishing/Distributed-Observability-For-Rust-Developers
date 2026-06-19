import { Component, effect, inject, input, signal } from '@angular/core';
import { ProductService } from './products.service';
import { CommonModule } from '@angular/common';
import { CartService } from '../services/cart.service';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'div.product-details',
  templateUrl: './product-details.component.html',
  styleUrls: ['./product-details.component.scss'],
  standalone: true,
  imports: [CommonModule, RouterLink],
})
export class ProductDetailsComponent {
  private productService = inject(ProductService);
  private cartService = inject(CartService);
  // Input signal for productId from route parameter
  readonly productId = input<string>('');

  // State signals
  readonly product = signal<Product | null>(null);
  readonly loading = signal<boolean>(false);
  readonly error = signal<string | null>(null);

  constructor() {
    // Effect to load product when productId changes
    effect(() => {
      const id = this.productId();
      if (id) {
        this.loadProduct(id);
      }
    });
  }

  private loadProduct(id: string): void {
    this.loading.set(true);
    this.error.set(null);

    this.productService.getProduct(id).subscribe({
      next: (product) => {
        this.product.set(product);
        this.loading.set(false);
      },
      error: (err) => {
        console.error('Failed to load product:', err);
        this.error.set('Failed to load product details. Please try again.');
        this.loading.set(false);
      },
    });
  }

  get discount(): number {
    const product = this.product() as Product;
    const initial = parseFloat(product.initial_price || '0');
    const discount = parseFloat(product.discount || '0');
    return initial > 0 ? Math.round((discount / initial) * 100) : 0;
  }

  addToCart(): void {
    const product = this.product();
    if (product && product.stock > 0) {
      // Convert Product to ProductCard format for cart service
      const productCard: ProductCard = {
        eid: product.eid,
        product_name: product.product_name,
        description: product.description,
        category_name: product.category_name,
        final_price: product.final_price,
        initial_price: product.initial_price,
        discount: product.discount,
        stock: product.stock,
        average_rating: null, // Not available in Product interface
        reviews_count: null, // Not available in Product interface
        brand: product.brand,
      };
      this.cartService.addToCart(productCard);
    }
  }
}
