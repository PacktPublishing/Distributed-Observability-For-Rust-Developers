import { HttpClient, HttpParams } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({
  providedIn: 'root',
})
export class ProductService {
  private http = inject(HttpClient);

  // Approach 1: Using TypeScript Decorators
  
  getProducts(page: number = 1, pageSize: number = 10): Observable<PaginatedResponse<ProductCard>> {
    const params = new HttpParams().set('page', page.toString()).set('page_size', pageSize.toString());

    return this.http.get<PaginatedResponse<ProductCard>>('/api/products', { params });
  }

  //@Busy({ message: 'Loading product details...' })
  getProduct(id: string): Observable<Product> {
    return this.http.get<Product>(`/api/products/${id}`);
  }

  //@Busy({ message: 'Searching products...' })
  searchProducts(query: string, page: number = 1): Observable<PaginatedResponse<ProductCard>> {
    const params = new HttpParams().set('q', query).set('page', page.toString());

    return this.http.get<PaginatedResponse<ProductCard>>('/api/products/search', { params });
  }
}
