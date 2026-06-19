import { Injectable, signal } from '@angular/core';

export type NotificationType = 'primary' | 'secondary' | 'success' | 'danger' | 'warning' | 'info' | 'light' | 'dark';

export interface Notification {
  id: number;
  message: string;
  type: NotificationType;
}

@Injectable({
  providedIn: 'root'
})
export class NotificationService {
  private notificationIdCounter = 0;
  private notifications = signal<Notification[]>([]);

  // Expose as readonly signal
  readonly notifications$ = this.notifications.asReadonly();

  /**
   * Show a notification that auto-dismisses after 3 seconds
   * @param message The message to display
   * @param type The alert type (default: 'info')
   */
  show(message: string, type: NotificationType = 'info'): void {
    const id = ++this.notificationIdCounter;
    const notification: Notification = { id, message, type };
    
    // Add notification to the list
    this.notifications.update(notifications => [...notifications, notification]);
    
    // Auto-dismiss after 3 seconds
    setTimeout(() => {
      this.dismiss(id);
    }, 3000);
  }

  /**
   * Manually dismiss a notification by ID
   * @param id The notification ID to dismiss
   */
  dismiss(id: number): void {
    this.notifications.update(notifications => 
      notifications.filter(n => n.id !== id)
    );
  }

  /**
   * Clear all notifications
   */
  clearAll(): void {
    this.notifications.set([]);
  }
}
