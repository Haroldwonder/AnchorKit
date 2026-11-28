/**
 * Internationalization (i18n) support for AnchorKit UI components.
 *
 * Provides a message catalog system with an I18nProvider context for
 * managing translations across components.
 */

import React, { createContext, useContext, useMemo, ReactNode } from 'react';

// ─── Types ────────────────────────────────────────────────────────────────

/** Supported locale codes */
export type Locale = 'en' | 'es' | 'fr' | 'de' | 'zh' | 'ja' | 'ar' | 'he' | string;

/** Message catalog key type for type-safe translations */
export type MessageKey =
  | 'common.loading'
  | 'common.error'
  | 'common.retry'
  | 'common.close'
  | 'common.done'
  | 'common.cancel'
  | 'common.confirm'
  | 'common.save'
  | 'common.delete'
  | 'common.edit'
  | 'common.copy'
  | 'common.copied'
  | 'common.reveal'
  | 'common.hide'
  | 'common.connect'
  | 'common.disconnect'
  | 'common.submit'
  | 'common.reset'
  | 'common.fetch'
  | 'common.sign'
  | 'common.verify'
  | 'common.authenticate'
  | 'common.authentication'
  | 'common.challenge'
  | 'common.token'
  | 'common.wallet'
  | 'common.network'
  | 'common.account'
  | 'common.protocol'
  | 'common.authMethod'
  | 'common.complete'
  | 'common.failed'
  | 'common.expired'
  | 'common.expiresIn'
  | 'common.validity'
  | 'common.status'
  | 'common.description'
  | 'common.timestamp'
  | 'common.details'
  | 'common.more'
  | 'common.less'
  | 'common.rawApiResponse'
  | 'common.noResponse'
  | 'common.noHistory'
  | 'common.clearHistory'
  | 'common.showHistory'
  | 'common.hideHistory'
  | 'common.endpoint'
  | 'common.method'
  | 'common.requestBody'
  | 'common.response'
  | 'common.curlCommand'
  | 'common.history'
  | 'common.time'
  | 'common.amount'
  | 'common.asset'
  | 'common.type'
  | 'common.id'
  | 'common.health'
  | 'common.healthy'
  | 'common.degraded'
  | 'common.unhealthy'
  | 'common.best'
  | 'common.selectAnchor'
  | 'common.noAnchors'
  | 'common.deposit'
  | 'common.withdrawal'
  | 'common.initiated'
  | 'common.pending'
  | 'common.processing'
  | 'common.completed'
  | 'common.refunded'
  | 'common.awaitingUser'
  | 'common.actionRequired'
  | 'common.estCompletion'
  | 'common.retryTransaction'
  | 'common.jwtReceived'
  | 'common.authSessionEstablished'
  | 'common.expiresIn24h'
  | 'common.connectWallet'
  | 'common.linkStellarKeypair'
  | 'common.privateKeyNeverLeaves'
  | 'common.fetchChallenge'
  | 'common.serverIssuesChallenge'
  | 'common.signChallenge'
  | 'common.proveKeypairOwnership'
  | 'common.submitGetToken'
  | 'common.bearerToken'
  | 'common.subsequentRequests'
  | 'common.challengeXdr'
  | 'common.signedXdr'
  | 'common.ed25519SignatureApplied'
  | 'common.receivedFrom'
  | 'common.jwtDecodedPayload'
  | 'common.sensitiveFields'
  | 'common.copyJwt'
  | 'common.revealToCopyJwt'
  | 'common.hideSensitiveFields'
  | 'common.revealSensitiveFields'
  | 'common.activityLog'
  | 'common.sessionReset'
  | 'common.tokenValidity'
  | 'common.sessionActive'
  | 'common.reauthenticateRequired'
  | 'common.liveSessionMonitor'
  | 'common.decodedPayload'
  | 'common.anchorDomain'
  | 'common.placeholderAnchorDomain'
  | 'common.errorInvalidJson'
  | 'common.errorFixJson'
  | 'common.errorSubmitRequest'
  | 'common.submitting'
  | 'common.submitRequest'
  | 'common.invalidJson'
  | 'common.enterValidJson'
  | 'common.copyEndpoint'
  | 'common.copyRequest'
  | 'common.copyResponse'
  | 'common.copyCurl'
  | 'common.clearHistoryConfirm'
  | 'common.historyCleared'
  | 'common.healthScore'
  | 'common.healthLabel'
  | 'common.endpointLabel'
  | 'common.nameLabel'
  | 'common.selected'
  | 'common.disabled'
  | 'common.useArrowKeys'
  | 'common.pressEnterToSelect'
  | 'common.authenticationFailed'
  | 'common.pleaseTryAgain'
  | 'common.challengeNotFetched'
  | 'common.challengeNotSigned'
  | 'common.requestingWalletConnection'
  | 'common.scanningWallets'
  | 'common.walletFound'
  | 'common.connectionApproved'
  | 'common.sendingChallengeToWallet'
  | 'common.userApprovedSignature'
  | 'common.transactionSigned'
  | 'common.postAuth'
  | 'common.sendingSignedXdr'
  | 'common.responseOk'
  | 'common.jwtReceivedCheck'
  | 'common.authSessionEstablishedCheck'
  | string;

/** Message catalog structure */
export interface MessageCatalog {
  [key: string]: string;
}

/** Complete translation messages for a locale */
export interface LocaleMessages {
  [locale: string]: MessageCatalog;
}

/** Default English messages */
export const defaultMessages: MessageCatalog = {
  'common.loading': 'Loading...',
  'common.error': 'Error',
  'common.retry': 'Retry',
  'common.close': 'Close',
  'common.done': 'Done',
  'common.cancel': 'Cancel',
  'common.confirm': 'Confirm',
  'common.save': 'Save',
  'common.delete': 'Delete',
  'common.edit': 'Edit',
  'common.copy': 'Copy',
  'common.copied': 'Copied!',
  'common.reveal': 'Reveal',
  'common.hide': 'Hide',
  'common.connect': 'Connect',
  'common.disconnect': 'Disconnect',
  'common.submit': 'Submit',
  'common.reset': 'Reset',
  'common.fetch': 'Fetch',
  'common.sign': 'Sign',
  'common.verify': 'Verify',
  'common.authenticate': 'Authenticate',
  'common.authentication': 'Authentication',
  'common.challenge': 'Challenge',
  'common.token': 'Token',
  'common.wallet': 'Wallet',
  'common.network': 'Network',
  'common.account': 'Account',
  'common.protocol': 'Protocol',
  'common.authMethod': 'Auth Method',
  'common.complete': 'Complete',
  'common.failed': 'Failed',
  'common.expired': 'Expired',
  'common.expiresIn': 'Expires in',
  'common.validity': 'Validity',
  'common.status': 'Status',
  'common.description': 'Description',
  'common.timestamp': 'Timestamp',
  'common.details': 'Details',
  'common.more': 'More',
  'common.less': 'Less',
  'common.rawApiResponse': 'Raw API Response',
  'common.noResponse': 'No response yet',
  'common.noHistory': 'No history yet',
  'common.clearHistory': 'Clear history',
  'common.showHistory': 'Show history',
  'common.hideHistory': 'Hide history',
  'common.endpoint': 'Endpoint',
  'common.method': 'Method',
  'common.requestBody': 'Request Body',
  'common.response': 'Response',
  'common.curlCommand': 'cURL Command',
  'common.history': 'History',
  'common.time': 'Time',
  'common.amount': 'Amount',
  'common.asset': 'Asset',
  'common.type': 'Type',
  'common.id': 'ID',
  'common.health': 'Health',
  'common.healthy': 'Healthy',
  'common.degraded': 'Degraded',
  'common.unhealthy': 'Unhealthy',
  'common.best': 'Best',
  'common.selectAnchor': 'Select Anchor',
  'common.noAnchors': 'No anchors available.',
  'common.deposit': 'Deposit',
  'common.withdrawal': 'Withdrawal',
  'common.initiated': 'Initiated',
  'common.pending': 'Pending',
  'common.processing': 'Processing',
  'common.completed': 'Completed',
  'common.refunded': 'Refunded',
  'common.awaitingUser': 'Action Required',
  'common.actionRequired': 'Action Required',
  'common.estCompletion': 'Est. Completion',
  'common.retryTransaction': 'Retry',
  'common.jwtReceived': 'JWT received',
  'common.authSessionEstablished': 'Auth session established',
  'common.expiresIn24h': 'expires in 24h',
  'common.connectWallet': 'Connect Wallet',
  'common.linkStellarKeypair': 'Link your Stellar keypair',
  'common.privateKeyNeverLeaves': 'Your private key never leaves your wallet.',
  'common.fetchChallenge': 'Fetch Challenge',
  'common.serverIssuesChallenge': 'Server issues a signed transaction',
  'common.signChallenge': 'Sign Challenge',
  'common.proveKeypairOwnership': 'Prove keypair ownership',
  'common.submitGetToken': 'Submit & Get Token',
  'common.bearerToken': 'Bearer token',
  'common.subsequentRequests': 'in all subsequent requests',
  'common.challengeXdr': 'CHALLENGE XDR',
  'common.signedXdr': 'SIGNED XDR',
  'common.ed25519SignatureApplied': 'ED25519 signature applied',
  'common.receivedFrom': 'Received from',
  'common.jwtDecodedPayload': 'DECODED PAYLOAD',
  'common.sensitiveFields': 'Sensitive fields hidden',
  'common.copyJwt': 'COPY JWT',
  'common.revealToCopyJwt': 'REVEAL TO COPY JWT',
  'common.hideSensitiveFields': 'Hide sensitive JWT fields',
  'common.revealSensitiveFields': 'Reveal sensitive JWT fields',
  'common.activityLog': 'Activity Log',
  'common.sessionReset': 'Session reset',
  'common.tokenValidity': 'TOKEN VALIDITY',
  'common.sessionActive': 'Session active',
  'common.reauthenticateRequired': 'Re-authenticate required',
  'common.liveSessionMonitor': 'Live session monitor',
  'common.decodedPayload': 'Decoded Payload',
  'common.anchorDomain': 'Anchor Domain',
  'common.placeholderAnchorDomain': 'anchor.example.com',
  'common.errorInvalidJson': 'Invalid JSON:',
  'common.errorFixJson': 'Fix JSON errors before submitting',
  'common.errorSubmitRequest': 'Submit request',
  'common.submitting': 'Submitting...',
  'common.submitRequest': 'Submit Request',
  'common.invalidJson': 'Invalid JSON',
  'common.enterValidJson': 'Enter valid JSON',
  'common.copyEndpoint': 'Copy endpoint',
  'common.copyRequest': 'Copy request',
  'common.copyResponse': 'Copy response',
  'common.copyCurl': 'Copy cURL',
  'common.clearHistoryConfirm': 'Clear history',
  'common.historyCleared': 'History cleared',
  'common.healthScore': 'Health Score',
  'common.healthLabel': 'Health',
  'common.endpointLabel': 'Endpoint',
  'common.nameLabel': 'Name',
  'common.selected': 'Selected',
  'common.disabled': 'Disabled',
  'common.useArrowKeys': 'Use arrow keys to navigate',
  'common.pressEnterToSelect': 'Press Enter to select',
  'common.authenticationFailed': 'Authentication failed. Please try again.',
  'common.pleaseTryAgain': 'Please try again.',
  'common.challengeNotFetched': 'Challenge not yet fetched',
  'common.challengeNotSigned': 'Challenge not yet signed',
  'common.requestingWalletConnection': 'Requesting wallet connection...',
  'common.scanningWallets': 'Scanning for available wallets...',
  'common.walletFound': 'Wallet found:',
  'common.connectionApproved': 'Connection approved',
  'common.sendingChallengeToWallet': 'Sending challenge XDR to wallet for signing...',
  'common.userApprovedSignature': 'User approved signature request',
  'common.transactionSigned': 'Transaction signed with ED25519 key',
  'common.postAuth': 'POST https://',
  'common.sendingSignedXdr': 'Sending signed XDR...',
  'common.responseOk': 'Response: 200 OK',
  'common.jwtReceivedCheck': 'JWT received',
  'common.authSessionEstablishedCheck': 'Auth session established — expires in 24h',
  'common.authStatus': 'Auth Status',
  'common.liveSessionMonitor': 'Live session monitor',
  'common.step': 'STEP',
};

/** Spanish translations */
export const esMessages: MessageCatalog = {
  'common.loading': 'Cargando...',
  'common.error': 'Error',
  'common.retry': 'Reintentar',
  'common.close': 'Cerrar',
  'common.done': 'Hecho',
  'common.cancel': 'Cancelar',
  'common.confirm': 'Confirmar',
  'common.save': 'Guardar',
  'common.delete': 'Eliminar',
  'common.edit': 'Editar',
  'common.copy': 'Copiar',
  'common.copied': '¡Copiado!',
  'common.reveal': 'Mostrar',
  'common.hide': 'Ocultar',
  'common.connect': 'Conectar',
  'common.disconnect': 'Desconectar',
  'common.submit': 'Enviar',
  'common.reset': 'Restablecer',
  'common.fetch': 'Obtener',
  'common.sign': 'Firmar',
  'common.verify': 'Verificar',
  'common.authenticate': 'Autenticar',
  'common.authentication': 'Autenticación',
  'common.challenge': 'Desafío',
  'common.token': 'Token',
  'common.wallet': 'Billetera',
  'common.network': 'Red',
  'common.account': 'Cuenta',
  'common.protocol': 'Protocolo',
  'common.authMethod': 'Método de autenticación',
  'common.complete': 'Completo',
  'common.failed': 'Fallido',
  'common.expired': 'Expirado',
  'common.expiresIn': 'Expira en',
  'common.validity': 'Validez',
  'common.status': 'Estado',
  'common.description': 'Descripción',
  'common.timestamp': 'Marca de tiempo',
  'common.details': 'Detalles',
  'common.more': 'Más',
  'common.less': 'Menos',
  'common.rawApiResponse': 'Respuesta API sin procesar',
  'common.noResponse': 'Sin respuesta aún',
  'common.noHistory': 'Sin historial aún',
  'common.clearHistory': 'Limpiar historial',
  'common.showHistory': 'Mostrar historial',
  'common.hideHistory': 'Ocultar historial',
  'common.endpoint': 'Punto final',
  'common.method': 'Método',
  'common.requestBody': 'Cuerpo de la solicitud',
  'common.response': 'Respuesta',
  'common.curlCommand': 'Comando cURL',
  'common.history': 'Historial',
  'common.time': 'Hora',
  'common.amount': 'Cantidad',
  'common.asset': 'Activo',
  'common.type': 'Tipo',
  'common.id': 'ID',
  'common.health': 'Salud',
  'common.healthy': 'Saludable',
  'common.degraded': 'Degradado',
  'common.unhealthy': 'No saludable',
  'common.best': 'Mejor',
  'common.selectAnchor': 'Seleccionar Anchor',
  'common.noAnchors': 'No hay anchors disponibles.',
  'common.deposit': 'Depósito',
  'common.withdrawal': 'Retiro',
  'common.initiated': 'Iniciado',
  'common.pending': 'Pendiente',
  'common.processing': 'Procesando',
  'common.completed': 'Completado',
  'common.refunded': 'Reembolsado',
  'common.awaitingUser': 'Acción requerida',
  'common.actionRequired': 'Acción requerida',
  'common.estCompletion': 'Est. finalización',
  'common.retryTransaction': 'Reintentar',
  'common.jwtReceived': 'JWT recibido',
  'common.authSessionEstablished': 'Sesión de autenticación establecida',
  'common.expiresIn24h': 'expira en 24h',
  'common.connectWallet': 'Conectar billetera',
  'common.linkStellarKeypair': 'Vincula tu par de claves Stellar',
  'common.privateKeyNeverLeaves': 'Tu clave privada nunca sale de tu billetera.',
  'common.fetchChallenge': 'Obtener desafío',
  'common.serverIssuesChallenge': 'El servidor emite una transacción firmada',
  'common.signChallenge': 'Firmar desafío',
  'common.proveKeypairOwnership': 'Demostrar propiedad del par de claves',
  'common.submitGetToken': 'Enviar y obtener token',
  'common.bearerToken': 'Token Bearer',
  'common.subsequentRequests': 'en todas las solicitudes posteriores',
  'common.challengeXdr': 'XDR DEL DESAFÍO',
  'common.signedXdr': 'XDR FIRMADO',
  'common.ed25519SignatureApplied': 'Firma ED25519 aplicada',
  'common.receivedFrom': 'Recibido de',
  'common.jwtDecodedPayload': 'CARGA ÚTIL DECODIFICADA',
  'common.sensitiveFields': 'Campos sensibles ocultos',
  'common.copyJwt': 'COPIAR JWT',
  'common.revealToCopyJwt': 'MOSTRAR PARA COPIAR JWT',
  'common.hideSensitiveFields': 'Ocultar campos sensibles del JWT',
  'common.revealSensitiveFields': 'Mostrar campos sensibles del JWT',
  'common.activityLog': 'Registro de actividad',
  'common.sessionReset': 'Sesión restablecida',
  'common.tokenValidity': 'VALIDEZ DEL TOKEN',
  'common.sessionActive': 'Sesión activa',
  'common.reauthenticateRequired': 'Se requiere re-autenticación',
  'common.liveSessionMonitor': 'Monitor de sesión en vivo',
  'common.decodedPayload': 'Carga útil decodificada',
  'common.anchorDomain': 'Dominio del Anchor',
  'common.placeholderAnchorDomain': 'anchor.ejemplo.com',
  'common.errorInvalidJson': 'JSON inválido:',
  'common.errorFixJson': 'Corrige los errores de JSON antes de enviar',
  'common.errorSubmitRequest': 'Enviar solicitud',
  'common.submitting': 'Enviando...',
  'common.submitRequest': 'Enviar solicitud',
  'common.invalidJson': 'JSON inválido',
  'common.enterValidJson': 'Ingrese JSON válido',
  'common.copyEndpoint': 'Copiar endpoint',
  'common.copyRequest': 'Copiar solicitud',
  'common.copyResponse': 'Copiar respuesta',
  'common.copyCurl': 'Copiar cURL',
  'common.clearHistoryConfirm': 'Limpiar historial',
  'common.historyCleared': 'Historial limpiado',
  'common.healthScore': 'Puntuación de salud',
  'common.healthLabel': 'Salud',
  'common.endpointLabel': 'Endpoint',
  'common.nameLabel': 'Nombre',
  'common.selected': 'Seleccionado',
  'common.disabled': 'Deshabilitado',
  'common.useArrowKeys': 'Use las flechas para navegar',
  'common.pressEnterToSelect': 'Presione Enter para seleccionar',
  'common.authenticationFailed': 'La autenticación falló. Inténtelo de nuevo.',
  'common.pleaseTryAgain': 'Por favor, inténtelo de nuevo.',
  'common.challengeNotFetched': 'Desafío aún no obtenido',
  'common.challengeNotSigned': 'Desafío aún no firmado',
  'common.requestingWalletConnection': 'Solicitando conexión de billetera...',
  'common.scanningWallets': 'Buscando billeteras disponibles...',
  'common.walletFound': 'Billetera encontrada:',
  'common.connectionApproved': 'Conexión aprobada',
  'common.sendingChallengeToWallet': 'Enviando XDR de desafío a la billetera para firmar...',
  'common.userApprovedSignature': 'Usuario aprobó la solicitud de firma',
  'common.transactionSigned': 'Transacción firmada con clave ED25519',
  'common.postAuth': 'POST https://',
  'common.sendingSignedXdr': 'Enviando XDR firmado...',
  'common.responseOk': 'Respuesta: 200 OK',
  'common.jwtReceivedCheck': 'JWT recibido',
  'common.authSessionEstablishedCheck': 'Sesión de autenticación establecida — expira en 24h',
  'common.authStatus': 'Estado de autenticación',
  'common.liveSessionMonitor': 'Monitor de sesión en vivo',
  'common.step': 'PASO',
};

/** Combined locale messages */
export const localeMessages: LocaleMessages = {
  en: defaultMessages,
  es: esMessages,
};

/** Type for the i18n context */
export interface I18nContextValue {
  /** Current locale */
  locale: Locale;
  /** Set the current locale */
  setLocale: (locale: Locale) => void;
  /** Translate a message key */
  t: (key: MessageKey, params?: Record<string, string | number>) => string;
  /** Check if a translation exists for a key */
  hasTranslation: (key: MessageKey) => boolean;
  /** Get all available locales */
  availableLocales: Locale[];
  /** Message catalog for current locale */
  messages: MessageCatalog;
}

/** Default context value (used when no provider) */
const defaultI18nContext: I18nContextValue = {
  locale: 'en',
  setLocale: () => {},
  t: (key: MessageKey) => defaultMessages[key] ?? key,
  hasTranslation: (key: MessageKey) => key in defaultMessages,
  availableLocales: ['en'],
  messages: defaultMessages,
};

/** I18n context */
export const I18nContext = createContext<I18nContextValue>(defaultI18nContext);

/** Props for I18nProvider */
export interface I18nProviderProps {
  /** Initial locale */
  locale?: Locale;
  /** Custom message catalogs */
  messages?: LocaleMessages;
  /** Child components */
  children: ReactNode;
  /** Fallback locale if translation missing */
  fallbackLocale?: Locale;
}

/**
 * I18nProvider - Provides internationalization context to child components.
 *
 * @example
 * ```tsx
 * <I18nProvider locale="es" messages={{ es: customSpanishMessages }}>
 *   <App />
 * </I18nProvider>
 * ```
 */
export function I18nProvider({
  locale = 'en',
  messages = localeMessages,
  children,
  fallbackLocale = 'en',
}: I18nProviderProps) {
  const [currentLocale, setCurrentLocale] = React.useState<Locale>(locale);

  const availableLocales = React.useMemo(
    () => Object.keys(messages) as Locale[],
    [messages],
  );

  const currentMessages = React.useMemo(
    () => messages[currentLocale] ?? messages[fallbackLocale] ?? defaultMessages,
    [messages, currentLocale, fallbackLocale],
  );

  const fallbackMessages = React.useMemo(
    () => messages[fallbackLocale] ?? defaultMessages,
    [messages, fallbackLocale],
  );

  const t = React.useCallback(
    (key: MessageKey, params?: Record<string, string | number>): string => {
      let translation = currentMessages[key] ?? fallbackMessages[key] ?? key;

      // Simple parameter interpolation: {param} -> value
      if (params) {
        Object.entries(params).forEach(([param, value]) => {
          translation = translation.replace(new RegExp(`\\{${param}\\}`, 'g'), String(value));
        });
      }

      return translation;
    },
    [currentMessages, fallbackMessages],
  );

  const hasTranslation = React.useCallback(
    (key: MessageKey): boolean => {
      return key in currentMessages || key in fallbackMessages;
    },
    [currentMessages, fallbackMessages],
  );

  const value = React.useMemo<I18nContextValue>(
    () => ({
      locale: currentLocale,
      setLocale: setCurrentLocale,
      t,
      hasTranslation,
      availableLocales,
      messages: currentMessages,
    }),
    [currentLocale, t, hasTranslation, availableLocales, currentMessages],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

/**
 * Hook to access i18n context.
 * Must be used within an I18nProvider.
 */
export function useI18n(): I18nContextValue {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error('useI18n must be used within an I18nProvider');
  }
  return context;
}

/**
 * Hook to get the translate function only.
 * Convenience hook for components that only need translation.
 */
export function useTranslate(): (key: MessageKey, params?: Record<string, string | number>) => string {
  const { t } = useI18n();
  return t;
}

export default I18nProvider;