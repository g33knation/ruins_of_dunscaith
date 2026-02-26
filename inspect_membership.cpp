
#include <iostream>
#include <cstddef>
#include <stdint.h>

#pragma pack(push, 1)

struct Membership_Entry_Struct
{
	uint32_t purchase_id;
	uint32_t bitwise_entry;
};

struct Membership_Setting_Struct
{
	uint32_t setting_index;
	uint32_t setting_id;
	int32_t setting_value;
};

struct Membership_Details_Struct
{
	uint32_t membership_setting_count;
	Membership_Setting_Struct settings[72];
	uint32_t race_entry_count;
	Membership_Entry_Struct membership_races[15];
	uint32_t class_entry_count;
	Membership_Entry_Struct membership_classes[15];
	uint32_t exit_url_length;
	uint32_t exit_url_length2;
};

struct Membership_Struct
{
	uint32_t membership;
	uint32_t races;
	uint32_t classes;
	uint32_t entrysize;
	int32_t entries[25];
};

#pragma pack(pop)

int main() {
    std::cout << "Membership_Details_Struct size: " << sizeof(Membership_Details_Struct) << std::endl;
    std::cout << "membership_setting_count offset: " << offsetof(Membership_Details_Struct, membership_setting_count) << std::endl;
    std::cout << "settings offset: " << offsetof(Membership_Details_Struct, settings) << std::endl;
    std::cout << "race_entry_count offset: " << offsetof(Membership_Details_Struct, race_entry_count) << std::endl;
    std::cout << "membership_races offset: " << offsetof(Membership_Details_Struct, membership_races) << std::endl;
    std::cout << "class_entry_count offset: " << offsetof(Membership_Details_Struct, class_entry_count) << std::endl;
    std::cout << "membership_classes offset: " << offsetof(Membership_Details_Struct, membership_classes) << std::endl;
    std::cout << "exit_url_length offset: " << offsetof(Membership_Details_Struct, exit_url_length) << std::endl;
    std::cout << "exit_url_length2 offset: " << offsetof(Membership_Details_Struct, exit_url_length2) << std::endl;

    std::cout << "Membership_Struct size: " << sizeof(Membership_Struct) << std::endl;
    std::cout << "membership offset: " << offsetof(Membership_Struct, membership) << std::endl;
    std::cout << "races offset: " << offsetof(Membership_Struct, races) << std::endl;
    std::cout << "classes offset: " << offsetof(Membership_Struct, classes) << std::endl;
    std::cout << "entrysize offset: " << offsetof(Membership_Struct, entrysize) << std::endl;
    std::cout << "entries offset: " << offsetof(Membership_Struct, entries) << std::endl;

    return 0;
}
