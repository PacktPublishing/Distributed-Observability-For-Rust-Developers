import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({
  providedIn: 'root',
})
export class OrderService {
  private http = inject(HttpClient);

  createOrder(orderRequest: CreateOrderRequest): Observable<{ success: boolean; order?: OrderResponse; error?: string; details?: string }> {
    return this.http.post<{ success: boolean; order?: OrderResponse; error?: string; details?: string }>('/api/orders', orderRequest);
  }
}
