import React from 'react';
import './EmptyState.css';

export interface EmptyStateProps {
  icon?: React.ReactNode;
  heading: string;
  subtext?: string;
  action?: React.ReactNode;
}

export function EmptyState({ icon, heading, subtext, action }: EmptyStateProps) {
  return (
    <div className="empty-state" role="status" aria-label={heading}>
      {icon && (
        <div className="empty-state__icon" aria-hidden="true">
          {icon}
        </div>
      )}
      <h3 className="empty-state__heading">{heading}</h3>
      {subtext && <p className="empty-state__subtext">{subtext}</p>}
      {action && <div className="empty-state__action">{action}</div>}
    </div>
  );
}
