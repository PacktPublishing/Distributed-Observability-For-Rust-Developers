import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { NotificationService } from './notification.service';

@Component({
  selector: 'div.notifications',
  standalone: true,
  imports: [CommonModule],
  template: `
      @for (notification of notificationService.notifications$(); track notification.id) {
        <div 
          class="alert alert-{{ notification.type }} alert-dismissible fade show mb-2 notification-item" 
          role="alert">
          {{ notification.message }}
          <button 
            type="button" 
            class="btn-close" 
            (click)="notificationService.dismiss(notification.id)"
            aria-label="Close">
          </button>
        </div>
      }
  `,
  styles: [`
    .notification-container {
      pointer-events: none;
    }
    
    .alert {
      pointer-events: auto;
      box-shadow: 0 4px 12px -2px rgba(0, 0, 0, 0.2);
      border-left-width: 4px;
      font-size: 0.95rem;
    }
    
    .notification-item {
      animation: slideIn 0.3s ease-out;
    }
    
    @keyframes slideIn {
      from {
        opacity: 0;
        transform: translateX(100%);
      }
      to {
        opacity: 1;
        transform: translateX(0);
      }
    }
  `]
})
export class NotificationComponent {
  notificationService = inject(NotificationService);
}
