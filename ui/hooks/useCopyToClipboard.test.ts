import { renderHook, act, waitFor } from '@testing-library/react';
import { useCopyToClipboard, formatJsonForCopy, generateCurlCommand, generateInstallCommand } from './useCopyToClipboard';

// Mock clipboard API
const mockWriteText = jest.fn();
Object.assign(navigator, {
  clipboard: {
    writeText: mockWriteText,
  },
});

describe('useCopyToClipboard', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.runOnlyPendingTimers();
    jest.useRealTimers();
  });

  it('should copy text to clipboard successfully', async () => {
    mockWriteText.mockResolvedValue(undefined);
    const { result } = renderHook(() => useCopyToClipboard());

    let copyResult: boolean = false;
    await act(async () => {
      copyResult = await result.current.copy('test text');
    });

    expect(copyResult).toBe(true);
    expect(mockWriteText).toHaveBeenCalledWith('test text');
    expect(result.current.isCopied).toBe(true);
    expect(result.current.copiedText).toBe('test text');
  });

  it('should reset copied state after duration', async () => {
    mockWriteText.mockResolvedValue(undefined);
    const { result } = renderHook(() => useCopyToClipboard({ successDuration: 1000 }));

    await act(async () => {
      await result.current.copy('test');
    });

    expect(result.current.isCopied).toBe(true);

    act(() => {
      jest.advanceTimersByTime(1000);
    });

    await waitFor(() => {
      expect(result.current.isCopied).toBe(false);
    });
  });

  it('should clear the reset timer on unmount (useEffect cleanup)', async () => {
    mockWriteText.mockResolvedValue(undefined);
    const { result, unmount } = renderHook(() => useCopyToClipboard({ successDuration: 2000 }));

    await act(async () => {
      await result.current.copy('test');
    });

    expect(result.current.isCopied).toBe(true);

    // Unmount before timer fires — should not cause state-update-on-unmounted-component warnings
    act(() => {
      unmount();
    });

    // Advancing time after unmount should not throw
    expect(() => {
      act(() => {
        jest.advanceTimersByTime(2000);
      });
    }).not.toThrow();
  });

  it('should call onSuccess callback', async () => {
    mockWriteText.mockResolvedValue(undefined);
    const onSuccess = jest.fn();
    const { result } = renderHook(() => useCopyToClipboard({ onSuccess }));

    await act(async () => {
      await result.current.copy('test');
    });

    expect(onSuccess).toHaveBeenCalledTimes(1);
  });

  it('should handle copy failure', async () => {
    const error = new Error('Copy failed');
    mockWriteText.mockRejectedValue(error);
    const onError = jest.fn();
    const { result } = renderHook(() => useCopyToClipboard({ onError }));

    let copyResult: boolean = false;
    await act(async () => {
      copyResult = await result.current.copy('test');
    });

    expect(copyResult).toBe(false);
    expect(onError).toHaveBeenCalledWith(error);
    expect(result.current.isCopied).toBe(false);
  });

  it('falls back to execCommand when clipboard.writeText rejects', async () => {
    mockWriteText.mockRejectedValue(new Error('denied'));
    const execCommand = jest.spyOn(document, 'execCommand').mockImplementation(() => true);
    const onError = jest.fn();
    const { result } = renderHook(() => useCopyToClipboard({ onError }));

    let copyResult = false;
    await act(async () => {
      copyResult = await result.current.copy('fallback-text');
    });

    expect(copyResult).toBe(true);
    expect(execCommand).toHaveBeenCalledWith('copy');
    expect(result.current.isCopied).toBe(true);
    expect(result.current.copiedText).toBe('fallback-text');
  });

  it('should reset state manually', async () => {
    mockWriteText.mockResolvedValue(undefined);
    const { result } = renderHook(() => useCopyToClipboard());

    await act(async () => {
      await result.current.copy('test');
    });

    expect(result.current.isCopied).toBe(true);

    act(() => {
      result.current.reset();
    });

    expect(result.current.isCopied).toBe(false);
    expect(result.current.copiedText).toBe(null);
  });

  it('should handle missing clipboard API', async () => {
    const originalClipboard = navigator.clipboard;
    // @ts-ignore
    delete navigator.clipboard;

    const onError = jest.fn();
    const { result } = renderHook(() => useCopyToClipboard({ onError }));

    let copyResult: boolean = false;
    await act(async () => {
      copyResult = await result.current.copy('test');
    });

    expect(copyResult).toBe(false);
    expect(onError).toHaveBeenCalled();

    // Restore
    Object.assign(navigator, { clipboard: originalClipboard });
  });
});

describe('formatJsonForCopy', () => {
  it('should format object as pretty JSON', () => {
    const data = { name: 'test', value: 123 };
    const result = formatJsonForCopy(data);
    expect(result).toBe('{\n  "name": "test",\n  "value": 123\n}');
  });

  it('should format object as compact JSON', () => {
    const data = { name: 'test', value: 123 };
    const result = formatJsonForCopy(data, false);
    expect(result).toBe('{"name":"test","value":123}');
  });

  it('should return string as-is', () => {
    const result = formatJsonForCopy('plain text');
    expect(result).toBe('plain text');
  });
});

describe('generateCurlCommand', () => {
  it('should generate basic GET curl command', () => {
    const result = generateCurlCommand({
      url: 'https://api.example.com/data',
    });
    expect(result).toBe('curl -X GET \\\n  "https://api.example.com/data"');
  });

  it('should generate POST curl with headers and body', () => {
    const result = generateCurlCommand({
      url: 'https://api.example.com/data',
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': 'Bearer token',
      },
      body: { name: 'test' },
    });

    expect(result).toContain('curl -X POST');
    expect(result).toContain('https://api.example.com/data');
    expect(result).toContain('-H "Content-Type: application/json"');
    expect(result).toContain('-H "Authorization: Bearer token"');
    expect(result).toContain('-d \'{\n  "name": "test"\n}\'');
  });

  it('should not include body for GET requests', () => {
    const result = generateCurlCommand({
      url: 'https://api.example.com/data',
      method: 'GET',
      body: { ignored: 'data' },
    });

    expect(result).not.toContain('-d');
  });

  it('should handle empty headers', () => {
    const result = generateCurlCommand({
      url: 'https://api.example.com/data',
      method: 'POST',
      body: { test: 'data' },
    });

    expect(result).toContain('curl -X POST');
    expect(result).toContain('-d');
    expect(result).not.toContain('-H');
  });
});

describe('generateInstallCommand', () => {
  it('should generate npm install command', () => {
    const result = generateInstallCommand('anchorkit', 'npm');
    expect(result).toBe('npm install anchorkit');
  });

  it('should generate yarn add command', () => {
    const result = generateInstallCommand('anchorkit', 'yarn');
    expect(result).toBe('yarn add anchorkit');
  });

  it('should generate pnpm add command', () => {
    const result = generateInstallCommand('anchorkit', 'pnpm');
    expect(result).toBe('pnpm add anchorkit');
  });

  it('should default to npm', () => {
    const result = generateInstallCommand('anchorkit');
    expect(result).toBe('npm install anchorkit');
  });
});
