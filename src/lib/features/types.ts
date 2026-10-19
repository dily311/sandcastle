
export interface FeatureRuntime {
    id: string;
    enabled: boolean;
    defaultEnabled: boolean;
    component?: any; 

    // 라이프 사이클
    init?: () => void | Promise<void>;
    shown?: () => void;
    disable?: () => void;
    destroy: () => void;

}

export interface Feature {
    id: string;
    name:string;
    icon: string;
    version: string;
    minAppVersion: string;
    description: string;
    // isDesktopOnly: boolean;
    defaultEnabled: boolean;
}
