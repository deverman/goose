import React, { createContext, useContext, useState } from 'react';
import { Message } from '../../types/message';
import { manageContextFromBackend, convertApiMessageToFrontendMessage } from './index';
import { generateSessionId, createContinuationSession } from './sessionManagement';
import {ChatType} from "../ChatView";


// Define the context management interface
interface ContextManagerState {
  summaryContent: string;
  summarizedThread: Message[];
  isSummaryModalOpen: boolean;
  isLoadingSummary: boolean;
  errorLoadingSummary: boolean;
}

interface ContextManagerActions {
  fetchSummary: (messages: Message[]) => Promise<void>;
  updateSummary: (newSummaryContent: string) => void;
  resetMessagesWithSummary: (
    messages: Message[],
    setMessages: (messages: Message[]) => void
  ) => void;
  openSummaryModal: () => void;
  closeSummaryModal: () => void;
  hasContextLengthExceededContent: (message: Message) => boolean;
  handleContextLengthExceeded: (messages: Message[], chatId: string, workingDir: string) => Promise<void>;
  setupContinuationChat: (setChat: (chat: ChatType) => void) => void;
  isContinuationSession: boolean;
}

// Create the context
const ContextManagerContext = createContext<
  (ContextManagerState & ContextManagerActions) | undefined
>(undefined);

// Create the provider component
export const ContextManagerProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [summaryContent, setSummaryContent] = useState<string>('');
  const [summarizedThread, setSummarizedThread] = useState<Message[]>([]);
  const [isSummaryModalOpen, setIsSummaryModalOpen] = useState<boolean>(false);
  const [isLoadingSummary, setIsLoadingSummary] = useState<boolean>(false);
  const [errorLoadingSummary, setErrorLoadingSummary] = useState<boolean>(false);
  const [isContinuationSession, setIsContinuationSession] = useState<boolean>(false);
  const [originalSessionId, setOriginalSessionId] = useState<string | null>(null);

  const handleContextLengthExceeded = async (
      messages: Message[],
      chatId: string,
      workingDir: string
  ): Promise<void> => {
    setIsLoadingSummary(true);
    setErrorLoadingSummary(false);

    try {
      // 1. First save the current session by creating a continuation session
      const continuationResult = await createContinuationSession({
        originalSessionId: chatId,
        messages: messages,
        workingDir: workingDir,
        title: `Session continued due to context length (${new Date().toLocaleString()})`
      });

      if (!continuationResult.success) {
        console.error('Failed to create continuation session:', continuationResult.error);
        throw new Error('Failed to create continuation session');
      }

      // Save the original session ID for reference
      setOriginalSessionId(chatId);
      setIsContinuationSession(true);

      // 2. Now get the summary from the backend
      const summaryResponse = await manageContextFromBackend({
        messages: messages,
        manageAction: 'summarize'
      });

      // Convert API messages to frontend messages
      const convertedMessages = summaryResponse.messages.map(apiMessage =>
          convertApiMessageToFrontendMessage(apiMessage)
      );

      // Extract summary from the first message
      const summaryMessage = convertedMessages[0].content[0];
      if (summaryMessage.type === 'text') {
        const summary = summaryMessage.text;
        setSummaryContent(summary);
        setSummarizedThread(convertedMessages);
      }

      setIsLoadingSummary(false);
    } catch (err) {
      console.error('Error handling context length exceeded:', err);
      setErrorLoadingSummary(true);
      setIsLoadingSummary(false);
    }
  };

  const setupContinuationChat = (setChat: (chat: ChatType) => void): void => {
    // Only proceed if we have a summarized thread
    if (summarizedThread.length === 0) return;

    // Create a new chat with the summary as the first message
    const newChatId = generateSessionId();

    setChat({
      id: newChatId,
      title: `Continued from ${originalSessionId || 'previous session'}`,
      messages: summarizedThread,
      messageHistoryIndex: summarizedThread.length
    });

    // Reset our internal state
    setSummarizedThread([]);
    setSummaryContent('');
    setIsContinuationSession(false);
    setOriginalSessionId(null);
  };

  const fetchSummary = async (messages: Message[]) => {
    setIsLoadingSummary(true);
    setErrorLoadingSummary(false);

    try {
      const response = await manageContextFromBackend({
        messages: messages,
        manageAction: 'summarize',
      });

      // Convert API messages to frontend messages
      const convertedMessages = response.messages.map((apiMessage) =>
        convertApiMessageToFrontendMessage(apiMessage)
      );

      // Extract the summary text from the first message
      const summaryMessage = convertedMessages[0].content[0];
      if (summaryMessage.type === 'text') {
        const summary = summaryMessage.text;
        setSummaryContent(summary);
        setSummarizedThread(convertedMessages);
      }

      setIsLoadingSummary(false);
    } catch (err) {
      console.error('Error fetching summary:', err);
      setErrorLoadingSummary(true);
      setIsLoadingSummary(false);
    }
  };

  const updateSummary = (newSummaryContent: string) => {
    // Update the summary content
    setSummaryContent(newSummaryContent);

    // Update the thread if it exists
    if (summarizedThread.length > 0) {
      // Create a deep copy of the thread
      const updatedThread = [...summarizedThread];

      // Create a copy of the first message
      const firstMessage = { ...updatedThread[0] };

      // Create a copy of the content array
      const updatedContent = [...firstMessage.content];

      // Update the summary text in the first content item
      if (updatedContent[0] && updatedContent[0].type === 'text') {
        updatedContent[0] = {
          ...updatedContent[0],
          text: newSummaryContent,
        };
      }

      // Update the message with the new content
      firstMessage.content = updatedContent;
      updatedThread[0] = firstMessage;

      // Update the thread
      setSummarizedThread(updatedThread);
    }
  };

  const resetMessagesWithSummary = (
    messages: Message[],
    setMessages: (messages: Message[]) => void
  ) => {
    // Update summarizedThread with metadata
    const updatedSummarizedThread = summarizedThread.map((msg) => ({
      ...msg,
      display: false,
      sendToLLM: true,
    }));

    // Update list of messages with other metadata
    const updatedMessages = messages.map((msg) => ({
      ...msg,
      display: true,
      sendToLLM: false,
    }));

    // Make a copy that combines both
    const newMessages = [...updatedMessages, ...updatedSummarizedThread];

    // Update the messages state
    setMessages(newMessages);

    // Clear the summarized thread and content
    setSummarizedThread([]);
    setSummaryContent('');
  };

  const hasContextLengthExceededContent = (message: Message): boolean => {
    return message.content.some((content) => content.type === 'contextLengthExceeded');
  };

  const openSummaryModal = () => {
    setIsSummaryModalOpen(true);
  };

  const closeSummaryModal = () => {
    setIsSummaryModalOpen(false);
  };

  const value = {
    // State
    summaryContent,
    summarizedThread,
    isSummaryModalOpen,
    isLoadingSummary,
    errorLoadingSummary,
    isContinuationSession,

    // Actions
    fetchSummary,
    updateSummary,
    resetMessagesWithSummary,
    openSummaryModal,
    closeSummaryModal,
    hasContextLengthExceededContent,
    handleContextLengthExceeded,
    setupContinuationChat
  };

  return <ContextManagerContext.Provider value={value}>{children}</ContextManagerContext.Provider>;
};

// Create a hook to use the context
export const useContextManager = () => {
  const context = useContext(ContextManagerContext);
  if (context === undefined) {
    throw new Error('useContextManager must be used within a ContextManagerProvider');
  }
  return context;
};
