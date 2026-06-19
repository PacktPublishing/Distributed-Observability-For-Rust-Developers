import { Component, effect, inject, input, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ProductService } from './products.service';
import { Router } from '@angular/router';
import { PaginateComponent } from './paginate.component';
import { ProductComponent } from './product.component';

@Component({
  selector: 'div.products-list',
  templateUrl: './products-list.component.html',
  styleUrls: ['./products-list.component.scss'],
  standalone: true,
  imports: [CommonModule, PaginateComponent, ProductComponent],
})
export class ProductsListComponent {
  private productService = inject(ProductService);
  private router = inject(Router);

  // Input signals automatically bound from query parameters
  readonly page = input<number, string | number>(1, {
    transform: (value: string | number) => {
      const num = typeof value === 'string' ? parseInt(value, 10) : value;
      return isNaN(num) ? 1 : Math.max(1, num);
    }
  });
  
  readonly pageSize = input<number, string | number>(10, {
    transform: (value: string | number) => {
      const num = typeof value === 'string' ? parseInt(value, 10) : value;
      return isNaN(num) || ![10, 20, 50, 100].includes(num) ? 10 : num;
    }
  });

  // State signals
  readonly products = signal<ProductCard[]>([]);
  readonly loading = signal<boolean>(false);
  readonly error = signal<string | null>(null);
  readonly totalPages = signal<number>(0);
  readonly total = signal<number>(0);

  constructor() {
    // Effect to load products when page or pageSize changes
    effect(() => {
      const currentPage = this.page();
      const currentPageSize = this.pageSize();
      this.loadProducts(currentPage, currentPageSize);
    });
  }

  private loadProducts(page: number, pageSize: number): void {
    this.loading.set(true);
    this.error.set(null);

    this.productService.getProducts(page, pageSize).subscribe({
      next: (response) => {
        this.products.set(response.products);
        this.totalPages.set(response.total_pages);
        this.total.set(response.total);
        this.loading.set(false);
      },
      error: (err) => {
        console.error('Failed to load products:', err);
        this.error.set('Failed to load products. Please try again.');
        this.products.set([]);
        this.loading.set(false);
      },
    });
  }
}
